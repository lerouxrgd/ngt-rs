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
    if env::var("CARGO_FEATURE_QUANTIZED").is_err() {
        config.define("NGT_QBG_DISABLED", "ON");
    } else {
        config.define("NGT_AVX2", "ON");
        config.define("CMAKE_BUILD_TYPE", "Release");
    }
    let dst = config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    #[cfg(not(feature = "static"))]
    {
        println!("cargo:rustc-link-lib=dylib=ngt");
    }
    #[cfg(feature = "static")]
    {
        cpp_build::Config::new()
            .include(format!("{}/lib", out_dir))
            .build("src/lib.rs");
        println!("cargo:rustc-link-lib=static=ngt");
        println!("cargo:rustc-link-lib=gomp");

        if env::var("CARGO_FEATURE_QUANTIZED").is_ok() {
            println!("cargo:rustc-link-lib=blas");
            println!("cargo:rustc-link-lib=lapack");
        }
    }

    let capi_header = if cfg!(feature = "quantized") {
        format!("{}/include/NGT/NGTQ/Capi.h", dst.display())
    } else {
        format!("{}/include/NGT/Capi.h", dst.display())
    };

    let out_path = PathBuf::from(out_dir);
    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}/include", dst.display()))
        .header(capi_header)
        .generate()
        .expect("Unable to generate bindings");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
