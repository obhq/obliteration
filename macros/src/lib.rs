use proc_macro::TokenStream;
use syn::{Error, ItemEnum, ItemStatic, LitStr, parse_macro_input};

mod bitflag;
mod elf;
mod enum_conversions;
mod errno;
mod vpath;

/// The reason we use `bitflag` as a name instead of `bitflags` is to make it matched with
/// `bitfield-struct` crate.
#[proc_macro_attribute]
pub fn bitflag(args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);
    let mut opts = self::bitflag::Options::default();
    let parser = syn::meta::parser(|m| opts.parse(m));

    parse_macro_input!(args with parser);

    self::bitflag::transform(opts, item)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

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
