use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{Error, Fields, ItemEnum, Path};

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
        let attrs = v.attrs;
        let ident = v.ident;
        let discriminant = match v.discriminant {
            Some(v) => v.1,
            None => {
                return Err(Error::new_spanned(
                    ident,
                    "auto-discriminant is not supported",
                ));
            }
        };

        if !matches!(v.fields, Fields::Unit) {
            return Err(Error::new_spanned(ident, "only unit variant is supported"));
        }

        body.extend(quote! {
            #(#attrs)*
            #[allow(non_upper_case_globals)]
            pub const #ident: Self = Self(#discriminant);
        });
    }

    // Generate methods.
    body.extend(quote! {
        /// Returns `true` if this set contains **any** flags in the `rhs` set.
        ///
        /// This performs the `&` operation on the underlying value and check if the results is
        /// non-zero.
        pub const fn has(self, rhs: Self) -> bool {
            (self.0 & rhs.0) != 0
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

        impl ::core::ops::BitOr for #impl_ident {
            type Output = Self;

            fn bitor(self, rhs: Self) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl ::core::ops::BitOrAssign for #impl_ident {
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0;
            }
        }
    })
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
