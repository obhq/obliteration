fn main() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    if os == "none" {
        println!("cargo::rustc-link-arg-bins=-zcommon-page-size=0x4000");
        println!("cargo::rustc-link-arg-bins=-zmax-page-size=0x4000");
    }
}
