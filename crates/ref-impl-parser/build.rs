fn main() {
    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .flag("-Wno-cpp")
        .file("reference_to_json/reference_to_json.cpp")
        .compile("reference_to_json");

    println!("cargo:rustc-link-lib=nixexpr");
    println!("cargo:rustc-link-lib=nixstore");
    println!("cargo:rustc-link-lib=nixutil");
}
