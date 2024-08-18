use proc_macro::TokenStream;
use syn::{parse_macro_input, Error, ItemEnum, ItemStatic, LitStr};

mod elf;
mod enum_conversions;
mod errno;
mod vpath;

/// Note will not produced for test target.
#[proc_macro_attribute]
pub fn elf_note(args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemStatic);
    let mut opts = self::elf::Options::default();
    let parser = syn::meta::parser(|m| opts.parse(m));

    parse_macro_input!(args with parser);

    self::elf::transform_note(opts, item)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(EnumConversions)]
pub fn implement_conversions(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);

    enum_conversions::transform(item)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Errno, attributes(errno))]
pub fn implement_errno(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);

    errno::transform(item)
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
