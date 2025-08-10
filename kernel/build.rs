fn main() {
    let target = std::env::var("TARGET").unwrap();

    match target.as_str() {
        "aarch64-unknown-none-softfloat" => println!("cargo::rustc-link-arg-bins=--pie"),
        _ => (),
    }

    println!("cargo::rustc-link-arg-bins=-zmax-page-size=0x4000");
}
