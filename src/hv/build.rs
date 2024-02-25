fn main() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    if os == "macos" {
        println!("cargo:rustc-link-lib=framework=Hypervisor");
    }
}
