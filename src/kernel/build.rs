fn main() {
    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => println!("cargo:rustc-link-lib=framework=Hypervisor"),
        _ => {}
    }
}
