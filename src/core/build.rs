use cbindgen::{Builder, Config, Language, Style};
use std::path::PathBuf;

fn main() {
    let core = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = core.parent().unwrap();
    let mut conf = Config::default();
    let mut buf = String::new();
    let externs = ["Param", "Pkg"];

    for ext in externs {
        buf.push_str("\nstruct ");
        buf.push_str(ext);
        buf.push(';');
    }

    conf.after_includes = Some(buf);
    conf.pragma_once = true;
    conf.language = Language::C;
    conf.cpp_compat = true;
    conf.style = Style::Tag;
    conf.usize_is_size_t = true;

    Builder::new()
        .with_crate(&core)
        .with_config(conf)
        .generate()
        .unwrap()
        .write_to_file(root.join("core.h"));
}
