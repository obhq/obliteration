use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::meta::ParseNestedMeta;
use syn::{parse_quote, Error, ItemStatic, LitInt, LitStr, Meta, StaticMutability, Type};

const OPT_SECTION: &'static str = "section";
const OPT_NAME: &'static str = "name";
const OPT_TY: &'static str = "ty";

pub fn transform_note(opts: Options, mut item: ItemStatic) -> syn::Result<TokenStream> {
    // Forbid "used" and "link_section" attribute.
    fn unsupported_attr(attr: impl ToTokens) -> syn::Result<TokenStream> {
        Err(Error::new_spanned(attr, "unsupported attribute"))
    }

    for attr in &item.attrs {
        match &attr.meta {
            Meta::Path(p) => {
                if p.is_ident("used") {
                    return unsupported_attr(p);
                }
            }
            Meta::List(_) => {}
            Meta::NameValue(a) => {
                if a.path.is_ident("link_section") {
                    return unsupported_attr(&a.path);
                }
            }
        }
    }

    // Disallow mutable.
    if let StaticMutability::Mut(t) = &item.mutability {
        return Err(Error::new_spanned(t, "mutable note is not supported"));
    }

    // Get section name.
    let section = match opts.section {
        Some(v) => v,
        None => {
            return Err(Error::new(
                Span::call_site(),
                format_args!("missing `{OPT_SECTION}` option"),
            ));
        }
    };

    // Get namespace.
    let mut name = match opts.name {
        Some(raw) => {
            let val = raw.value();

            if val.contains('\0') {
                return Err(Error::new_spanned(
                    raw,
                    "note name cannot contains NUL character",
                ));
            }

            val
        }
        None => {
            return Err(Error::new(
                Span::call_site(),
                format_args!("missing `{OPT_NAME}` option"),
            ));
        }
    };

    name.push('\0');

    // Get type
    let ty: u32 = match opts.ty {
        Some(v) => v.base10_parse()?,
        None => {
            return Err(Error::new(
                Span::call_site(),
                format_args!("missing `{OPT_TY}` option"),
            ));
        }
    };

    // Replace type.
    let nlen = name.len();
    let dlen = match item.ty.as_ref() {
        Type::Array(arr) => match arr.elem.as_ref() {
            Type::Path(elem) if elem.qself.is_none() && elem.path.is_ident("u8") => &arr.len,
            t => return Err(Error::new_spanned(t, "expect `u8`")),
        },
        t => return Err(Error::new_spanned(t, "expect array of `u8`")),
    };

    item.ty = parse_quote!(crate::imgfmt::elf::Note<#nlen, { #dlen }>);

    // Replace value.
    let name = name.as_bytes();
    let desc = item.expr;

    item.expr = parse_quote!(unsafe { crate::imgfmt::elf::Note::new([#(#name),*], #ty, #desc) });

    // Compose.
    Ok(quote! {
        #[used]
        #[link_section = #section]
        #item
    })
}

#[derive(Default)]
pub struct Options {
    section: Option<LitStr>,
    name: Option<LitStr>,
    ty: Option<LitInt>,
}

impl Options {
    pub fn parse(&mut self, m: ParseNestedMeta) -> syn::Result<()> {
        if m.path.is_ident(OPT_SECTION) {
            self.section = Some(m.value()?.parse()?);
        } else if m.path.is_ident(OPT_NAME) {
            self.name = Some(m.value()?.parse()?);
        } else if m.path.is_ident(OPT_TY) {
            self.ty = Some(m.value()?.parse()?);
        } else {
            return Err(m.error("unknown option"));
        }

        Ok(())
    }
}
