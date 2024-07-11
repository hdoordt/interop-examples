fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/main.cc")
        .compile("cxx-pretty-json");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/main.cc");
}
