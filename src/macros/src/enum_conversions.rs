use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemEnum;

pub fn transform(arg: ItemEnum) -> syn::Result<TokenStream> {
    let enum_name = &arg.ident;

    let mut stream = TokenStream::new();

    for variant in arg.variants.iter() {
        if variant.fields.len() != 1 {
            return Err(syn::Error::new_spanned(
                variant,
                "expected variant to have one field",
            ));
        }

        let field = variant.fields.iter().next().unwrap();

        let variant_name = &variant.ident;
        let variant_type = &field.ty;

        let impls = quote! {
            impl From<#variant_type> for #enum_name {
                fn from(v: #variant_type) -> Self {
                    Self::#variant_name(v)
                }
            }

            impl TryFrom<#enum_name> for #variant_type {
                type Error = #enum_name;

                fn try_from(v: #enum_name) -> Result<Self, Self::Error> {
                    match v {
                        #enum_name::#variant_name(v) => Ok(v),
                        _ => Err(v),
                    }
                }
            }

            impl<'a> TryFrom<&'a #enum_name> for &'a #variant_type {
                type Error = &'a #enum_name;

                fn try_from(v: &'a #enum_name) -> Result<Self, Self::Error> {
                    match v {
                        #enum_name::#variant_name(v) => Ok(v),
                        _ => Err(v),
                    }
                }
            }

            impl<'a> TryFrom<&'a mut #enum_name> for &'a mut #variant_type {
                type Error = &'a mut #enum_name;

                fn try_from(v: &'a mut #enum_name) -> Result<Self, Self::Error> {
                    match v {
                        #enum_name::#variant_name(v) => Ok(v),
                        _ => Err(v),
                    }
                }
            }
        };

        stream.extend(impls);
    }

    let expanded = quote! {
        #arg

        #stream
    };

    Ok(expanded)
}
