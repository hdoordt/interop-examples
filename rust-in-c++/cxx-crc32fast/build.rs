fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/crc32fast.cc")
        .compile("cxx-crc32fast");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/crc32fast.cc");
    println!("cargo:rerun-if-changed=include/crc32fast.h");
}
