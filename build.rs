fn main() {
    println!("cargo:rustc-link-lib=nixexpr");
    println!("cargo:rustc-link-lib=nixstore");
    println!("cargo:rustc-link-lib=nixutil");
    
    cc::Build::new()
        .cpp(true)
        .cpp_link_stdlib("stdc++")
        .file("reference_to_json/reference_to_json.cpp")
        .compile("foo");
}