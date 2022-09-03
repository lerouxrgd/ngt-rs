use std::env;
use std::path::PathBuf;

fn main() {
    let mut config = cmake::Config::new("NGT");

    if env::var("CARGO_FEATURE_SHARED_MEM").is_ok() {
        config.define("NGT_SHARED_MEMORY_ALLOCATOR", "ON");
    }

    if env::var("CARGO_FEATURE_LARGE_DATA").is_ok() {
        config.define("NGT_LARGE_DATASET", "ON");
    }

    let dst = config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=dylib=ngt");

    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}/include", dst.display()))
        .header(format!("{}/include/NGT/NGTQ/Capi.h", dst.display()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
