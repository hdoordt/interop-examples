fn main() {
    println!("cargo:rerun-if-changed=tweetnacl.c");
    println!("cargo:rerun-if-changed=tweetnacl.h");

    let bindings = bindgen::builder()
        .header("tweetnacl.h")
        .generate()
        .expect("Unable to generate bindings to tweetnacl.h");

    let out_path = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_path);
    bindings
        .write_to_file(out_path.join("tweetnacl_bindings.rs"))
        .expect("Couldn't write bindings to tweetnacl.h!");

    cc::Build::new()
        .warnings(false)
        .extra_warnings(false)
        .file("tweetnacl.c")
        .compile("tweetnacl");
}
