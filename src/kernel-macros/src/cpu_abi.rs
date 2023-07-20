use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::{Error, ItemFn};

pub fn transform(item: ItemFn) -> syn::Result<TokenStream> {
    let span = item.span();

    if item.sig.abi.is_some() {
        return Err(Error::new(span, "expected a function without `extern`"));
    } else if item.sig.asyncness.is_some() {
        return Err(Error::new(span, "expected non-async function"));
    } else if item.sig.constness.is_some() {
        return Err(Error::new(span, "expected non-const function"));
    } else if item.sig.generics.lt_token.is_some() {
        return Err(Error::new(span, "expected non-generic function"));
    } else if item.sig.variadic.is_some() {
        return Err(Error::new(span, "expected non-variadic function"));
    }

    #[cfg(target_arch = "x86_64")]
    let abi = "sysv64";
    #[cfg(target_arch = "aarch64")]
    let abi = "C";
    let attrs = item.attrs;
    let vis = item.vis;
    let safety = item.sig.unsafety;
    let name = item.sig.ident;
    let args = item.sig.inputs;
    let ret = item.sig.output;
    let block = item.block;

    Ok(quote_spanned! { span =>
        #(#attrs)*
        #vis #safety extern #abi fn #name(#args) #ret {
            #block
        }
    })
}
