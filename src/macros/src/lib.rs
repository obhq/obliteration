use proc_macro::TokenStream;
use syn::{parse_macro_input, Error, ItemFn, LitStr};

mod cpu_abi;
mod vpath;

/// Add `extern "sysv64"` on x86-64 or `extern "aapcs"` on AArch64.
#[proc_macro_attribute]
pub fn cpu_abi(_: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);

    cpu_abi::transform(item)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn vpath(arg: TokenStream) -> TokenStream {
    let arg = parse_macro_input!(arg as LitStr);

    vpath::transform(arg)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
