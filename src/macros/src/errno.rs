use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{punctuated::Punctuated, Fields, ItemEnum, Meta, Token, Variant};

pub fn transform(arg: ItemEnum) -> syn::Result<TokenStream> {
    let enum_name = &arg.ident;

    let arms = arg
        .variants
        .iter()
        .map(|variant| process_variant(variant, enum_name))
        .collect::<Result<Vec<_>, _>>()?;

    let res = quote!(
        impl Errno for #enum_name {
            fn errno(&self) -> std::num::NonZeroI32 {
                match self {
                    #(#arms)*
                }
            }
        }
    );

    Ok(res)
}

fn process_variant(variant: &Variant, enum_name: &Ident) -> syn::Result<TokenStream> {
    for attr in variant.attrs.iter() {
        if attr.path().is_ident("errno") {
            let meta = attr
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                .into_iter()
                .collect::<Vec<_>>();

            match meta.as_slice() {
                [Meta::Path(path)] => {
                    let errno = path.get_ident().ok_or_else(|| {
                        syn::Error::new_spanned(
                            attr,
                            "incorrect errno usage. Correct is #[errno(...)]",
                        )
                    })?;

                    let variant_name = &variant.ident;

                    let arm = match variant.fields {
                        Fields::Unit => quote!(#enum_name::#variant_name => #errno,),
                        Fields::Named(_) => quote!(#enum_name::#variant_name {..} => #errno,),
                        Fields::Unnamed(_) => quote!(#enum_name::#variant_name (..) => #errno,),
                    };

                    return Ok(arm);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "incorrect errno usage. Correct is #[errno(...)]",
                    ))
                }
            }
        }
    }

    Err(syn::Error::new_spanned(
        variant,
        "variant does not have an errno attribute",
    ))
}
