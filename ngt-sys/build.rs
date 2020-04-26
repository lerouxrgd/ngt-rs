use std::env;
use std::path::PathBuf;

fn main() {
    let mut config = cmake::Config::new("NGT");
    let dst = config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=dylib=ngt");

    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/NGT/Capi.h", dst.display()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
