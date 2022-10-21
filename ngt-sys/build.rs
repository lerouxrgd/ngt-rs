use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut config = cmake::Config::new("NGT");

    if env::var("CARGO_FEATURE_SHARED_MEM").is_ok() {
        config.define("NGT_SHARED_MEMORY_ALLOCATOR", "ON");
    }

    if env::var("CARGO_FEATURE_LARGE_DATA").is_ok() {
        config.define("NGT_LARGE_DATASET", "ON");
    }

    let dst = config.build();

    #[cfg(feature = "static")]
    cpp_build::Config::new()
        .include(format!("{}/lib", out_dir))
        .build("src/lib.rs");

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    #[cfg(feature = "static")]
    println!("cargo:rustc-link-lib=static=ngt");
    #[cfg(not(feature = "static"))]
    println!("cargo:rustc-link-lib=dylib=ngt");

    #[cfg(feature = "static")]
    println!("cargo:rustc-link-lib=gomp");

    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}/include", dst.display()))
        .header(format!("{}/include/NGT/NGTQ/Capi.h", dst.display()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
