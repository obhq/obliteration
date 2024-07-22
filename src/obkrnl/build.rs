fn main() {
    println!("cargo::rustc-link-arg-bin=obkrnl=-zcommon-page-size=0x4000");
    println!("cargo::rustc-link-arg-bin=obkrnl=-zmax-page-size=0x4000");
}
