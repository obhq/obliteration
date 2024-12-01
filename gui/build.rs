use slint_build::CompilerConfiguration;
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());

    // Compile Slint.
    let config = CompilerConfiguration::new()
        .with_style(String::from("fluent-dark"))
        .with_library_paths(HashMap::from([("root".into(), root.join("ui"))]));

    slint_build::compile_with_config(PathBuf::from_iter(["ui", "main.slint"]), config).unwrap();

    // Compile resources.rc.
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_resource_file(root.join("resources.rc").to_str().unwrap())
        .compile();
}
