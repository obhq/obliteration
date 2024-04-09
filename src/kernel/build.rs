fn main() {
    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "linux" | "android" => {
            println!("cargo::rerun-if-changed=src/hv/linux/kvm.cpp");

            cc::Build::new()
                .cpp(true)
                .file("src/hv/linux/kvm.cpp")
                .compile("obkrnlffi");
        }
        "macos" => println!("cargo:rustc-link-lib=framework=Hypervisor"),
        _ => {}
    }
}
