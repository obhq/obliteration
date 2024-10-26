use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{Error, LitStr};

pub fn transform(arg: LitStr) -> syn::Result<TokenStream> {
    // TODO: Merge this with VPath::is_valid().
    let span = arg.span();
    let arg = arg.value();

    if arg.is_empty() {
        return Err(Error::new(span, "cannot be an empty string"));
    } else if !arg.starts_with('/') {
        return Err(Error::new(span, "cannot begin with `/`"));
    } else if arg.ends_with('/') {
        return Err(Error::new(span, "cannot end with `/`"));
    }

    // Check thoroughly.
    let mut sep = 0;

    for (i, ch) in arg.bytes().enumerate() {
        if i == 0 || ch != b'/' {
            continue;
        }

        // Disallow a consecutive of the separator, "." and "..".
        let com = &arg[(sep + 1)..i];

        if com.is_empty() {
            return Err(Error::new(span, "cannot contains consecutive of `/`"));
        } else if com == "." {
            return Err(Error::new(span, "cannot contains `/./`"));
        } else if com == ".." {
            return Err(Error::new(span, "cannot contains `/../`"));
        }

        sep = i;
    }

    Ok(quote_spanned!(span=> unsafe { crate::fs::VPath::new_unchecked(#arg) }))
}
