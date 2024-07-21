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

    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "linux" => conf.sys_includes.push("linux/kvm.h".into()),
        _ => {}
    }

    conf.after_includes = Some(buf);
    conf.pragma_once = true;
    conf.language = Language::C;
    conf.cpp_compat = true;
    conf.style = Style::Tag;
    conf.usize_is_size_t = true;
    conf.export.exclude.push("KvmRegs".into());
    conf.export
        .rename
        .insert("KvmRegs".into(), "kvm_regs".into());
    conf.defines
        .insert("target_os = linux".into(), "__linux__".into());
    conf.defines
        .insert("target_os = macos".into(), "__APPLE__".into());

    Builder::new()
        .with_crate(&core)
        .with_config(conf)
        .generate()
        .unwrap()
        .write_to_file(root.join("core.h"));
}
