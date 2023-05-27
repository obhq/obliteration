use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, ItemFn};

pub fn transform(item: ItemFn) -> syn::Result<TokenStream> {
    if item.sig.abi.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "expected a function without `extern`",
        ));
    } else if !item.attrs.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "expected a function without other attributes",
        ));
    } else if item.sig.asyncness.is_some() {
        return Err(Error::new(Span::call_site(), "expected non-async function"));
    } else if item.sig.constness.is_some() {
        return Err(Error::new(Span::call_site(), "expected non-const function"));
    } else if item.sig.generics.lt_token.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "expected non-generic function",
        ));
    } else if item.sig.variadic.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "expected non-variadic function",
        ));
    }

    #[cfg(target_arch = "x86_64")]
    let abi = "sysv64";
    #[cfg(target_arch = "aarch64")]
    let abi = "aapcs";
    let vis = item.vis;
    let safety = item.sig.unsafety;
    let name = item.sig.ident;
    let args = item.sig.inputs;
    let ret = item.sig.output;
    let block = item.block;

    Ok(quote! {
        #vis extern #abi #safety fn #name(#args) #ret {
            #block
        }
    })
}
