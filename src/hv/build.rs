fn main() {
    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "linux" | "android" => cc::Build::new()
            .cpp(true)
            .file("src/linux/kvm.cpp")
            .compile("hvkvm"),
        "macos" => println!("cargo:rustc-link-lib=framework=Hypervisor"),
        _ => {}
    }
}
