use slint_build::CompilerConfiguration;
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // Build path for @root.
    let mut root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());

    root.push("ui");

    // Compile Slint.
    let config = CompilerConfiguration::new()
        .with_style(String::from("fluent-dark"))
        .with_library_paths(HashMap::from([("root".into(), root)]));

    slint_build::compile_with_config(PathBuf::from_iter(["ui", "main.slint"]), config).unwrap();
}
