use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use std::num::NonZero;
use syn::meta::ParseNestedMeta;
use syn::punctuated::Pair;
use syn::{Error, Expr, ExprLit, Fields, ItemEnum, Lit, LitInt, Path, Type, parse_quote};

pub fn transform(opts: Options, item: ItemEnum) -> syn::Result<TokenStream> {
    let ty = opts
        .ty
        .as_ref()
        .ok_or_else(|| Error::new(Span::call_site(), "missing underlying type name"))?;
    let ident = item.ident;

    if item.generics.lt_token.is_some() {
        return Err(Error::new_spanned(ident, "generic enum is not supported"));
    }

    // Parse body.
    let mut body = TokenStream::new();

    for v in item.variants {
        // Parse discriminant.
        let ident = v.ident;
        let (mask, bits, dis) = match v.discriminant {
            Some((
                _,
                Expr::Lit(ExprLit {
                    attrs: _,
                    lit: Lit::Int(v),
                }),
            )) => parse_discriminant(ty, v)?,
            Some((_, v)) => {
                return Err(Error::new_spanned(
                    v,
                    "discriminant other than integer literal is not supported",
                ));
            }
            None => {
                return Err(Error::new_spanned(
                    ident,
                    "auto-discriminant is not supported",
                ));
            }
        };

        // Generate flag.
        let attrs = v.attrs;
        let ty = match v.fields {
            Fields::Named(_) => {
                return Err(Error::new_spanned(
                    ident,
                    "variant with named fields is not supported",
                ));
            }
            Fields::Unnamed(mut v) => {
                // Get field.
                let f = match v.unnamed.pop() {
                    Some(Pair::End(v)) => v,
                    Some(_) => {
                        return Err(Error::new_spanned(
                            ident,
                            "variant with multiple fields is not supported",
                        ));
                    }
                    None => {
                        return Err(Error::new_spanned(
                            ident,
                            "field-less variant is not supported",
                        ));
                    }
                };

                f.ty
            }
            Fields::Unit => parse_quote!(bool),
        };

        if let Type::Path(t) = &ty {
            if t.qself.is_none() && t.path.is_ident("bool") && bits.get() != 1 {
                return Err(Error::new_spanned(
                    dis,
                    "multiple bits for a boolean mask is not supported",
                ));
            }
        }

        body.extend(quote! {
            #(#attrs)*
            #[allow(non_upper_case_globals)]
            pub const #ident: ::bitflag::Mask<Self, #ty> = unsafe { ::bitflag::Mask::new(#mask) };
        });
    }

    // Generate methods.
    body.extend(quote! {
        /// Returns a new set with all bits of the backed-storage set to zero.
        pub const fn zeroed() -> Self {
            Self(0)
        }

        /// Returns `true` if this set contains **any** flags in the `rhs` set.
        ///
        /// This performs the `&` operation on the underlying value and check if the results is
        /// non-zero.
        pub fn has_any(self, rhs: impl Into<Self>) -> bool {
            (self.0 & rhs.into().0) != 0
        }

        /// Returns `true` if this set contains **all** flags in the `rhs` set.
        ///
        /// This performs the `&` operation on the underlying value and check if the results is
        /// equal to `rhs`.
        pub const fn has_all(self, rhs: Self) -> bool {
            (self.0 & rhs.0) == rhs.0
        }
    });

    // Compose.
    let attrs = item.attrs;
    let vis = item.vis;
    let mut impl_ident = ident.clone();

    impl_ident.set_span(Span::call_site());

    Ok(quote! {
        #(#attrs)*
        #[repr(transparent)]
        #[derive(Clone, Copy)]
        #vis struct #ident(#ty);

        impl #impl_ident {
            #body
        }

        impl ::bitflag::Type for #impl_ident {
            type Raw = #ty;
        }

        impl From<#ty> for #impl_ident {
            fn from(value: #ty) -> Self {
                Self(value)
            }
        }

        impl From<::bitflag::Mask<Self, bool>> for #impl_ident {
            fn from(value: ::bitflag::Mask<Self, bool>) -> Self {
                Self(value.mask())
            }
        }

        impl ::core::ops::BitOr<::bitflag::Mask<Self, bool>> for #impl_ident {
            type Output = Self;

            fn bitor(self, rhs: ::bitflag::Mask<Self, bool>) -> Self::Output {
                Self(self.0 | rhs.mask())
            }
        }

        impl ::core::ops::BitOrAssign for #impl_ident {
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0;
            }
        }

        impl ::core::ops::BitOrAssign<::bitflag::Mask<Self, bool>> for #impl_ident {
            fn bitor_assign(&mut self, rhs: ::bitflag::Mask<Self, bool>) {
                self.0 |= rhs.mask();
            }
        }

        impl ::core::ops::BitAnd for #impl_ident {
            type Output = Self;

            fn bitand(self, rhs: Self) -> Self::Output {
                Self(self.0 & rhs.0)
            }
        }

        impl From<#impl_ident> for #ty {
            fn from(value: #impl_ident) -> Self {
                value.0
            }
        }
    })
}

fn parse_discriminant(ty: &Path, dis: LitInt) -> syn::Result<(TokenStream, NonZero<u32>, LitInt)> {
    let v = if ty.is_ident("u32") {
        let v = dis.base10_parse::<u32>()?;
        let i = v.trailing_zeros();
        let mut r = v >> i;

        // Disallow zero value.
        if r == 0 {
            return Err(Error::new_spanned(
                dis,
                "zero discriminant is not supported",
            ));
        }

        // Disallow zero bit in the middle.
        let mut n = 0;

        while r != 0 {
            if r & 1 == 0 {
                return Err(Error::new_spanned(
                    dis,
                    "discriminant with non-contiguous bits is not supported",
                ));
            }

            n += 1;
            r >>= 1;
        }

        (v.into_token_stream(), n.try_into().unwrap(), dis)
    } else {
        return Err(Error::new_spanned(ty, "unsupported underlying type"));
    };

    Ok(v)
}

#[derive(Default)]
pub struct Options {
    ty: Option<Path>,
}

impl Options {
    pub fn parse(&mut self, m: ParseNestedMeta) -> syn::Result<()> {
        if self.ty.is_none() {
            self.ty = Some(m.path);
        }

        Ok(())
    }
}
