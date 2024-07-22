fn main() {
    println!("cargo::rustc-link-arg-bins=-zcommon-page-size=0x4000");
    println!("cargo::rustc-link-arg-bins=-zmax-page-size=0x4000");
}
