use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{punctuated::Punctuated, Field, Fields, ItemEnum, Meta, Token, Variant};

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
                match *self {
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

    // If we are here, that means we haven't found any errno attributes
    // Now we try to check the data to find a field marked with
    // either #[source] or #[from] (which belong to the thiserror crate)

    match &variant.fields {
        Fields::Named(_) => todo!("Named fields are not supported yet"),
        Fields::Unnamed(fields) => {
            let mut pos = None;

            fields
                .unnamed
                .iter()
                .enumerate()
                .try_for_each(|(i, field)| {
                    for attr in field.attrs.iter() {
                        if attr.path().is_ident("source") || attr.path().is_ident("from") {
                            if let Some(_) = pos.replace(i) {
                                return Err(syn::Error::new_spanned(
                                    attr,
                                    format!(
                                        "multiple fields marked with either #[source] or #[from] found. \
                                        Only one field is allowed"
                                    ),
                                ))
                            }
                        }

                    }

                    Ok(())
                })?;

            return match pos {
                Some(pos) => {
                    let variant_name = &variant.ident;

                    Ok(quote!(
                        #enum_name::#variant_name (e) => e.errno()))
                }
                None => Err(syn::Error::new_spanned(
                    variant,
                    "no fields of this variant are marked with either #[source] or #[from]",
                )),
            };
        }
        Fields::Unit => {
            return Err(syn::Error::new_spanned(
                variant,
                "no errno attribute found on a variant with no fields",
            ))
        }
    }

    Err(syn::Error::new_spanned(
        variant,
        "variant does not have an errno attribute or any inner error to get the errno from",
    ))
}
