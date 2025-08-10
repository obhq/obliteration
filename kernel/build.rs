fn main() {
    // Enable PIE on target that we need to build std. We need to check a triple here because this
    // configuration is per-target.
    let target = std::env::var("TARGET").unwrap();

    if target == "aarch64-unknown-none-softfloat" {
        println!("cargo::rustc-link-arg-bins=--pie");
    }

    // Set max-page-size.
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    if os == "none" {
        println!("cargo::rustc-link-arg-bins=-zmax-page-size=0x4000");
    }
}
