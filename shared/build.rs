fn main() {
    let header = "ksu-capstone-embedded/shared/shared_lib.h";

    println!("cargo:rerun-if-changed={header}");

    let bindings = bindgen::Builder::default()
        .header(header)
        .derive_debug(true)
        .derive_default(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .generate()
        .expect("Unable to generate bindings from shared_lib.h");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
