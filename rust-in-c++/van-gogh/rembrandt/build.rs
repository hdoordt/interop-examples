fn main() {
    cxx_build::bridge("src/lib.rs") // returns a cc::Build
        .std("c++11")
        .include("src")
        
        .compile("rembrandt");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
