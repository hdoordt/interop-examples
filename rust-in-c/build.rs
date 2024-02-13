use std::{env, path::Path};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    std::fs::remove_dir_all("./bindings").ok();
    std::fs::create_dir_all("./bindings").unwrap();
    
    // Invoke cbindgen
    cbindgen::Builder::new()
      .with_crate(crate_dir)
      .with_language(cbindgen::Language::C)
      .generate()
      .expect("Unable to generate bindings")
      .write_to_file("bindings/rust-in-c.h");

    // Invoke diplomat
    diplomat_tool::gen(
      Path::new("src/lib.rs"),
      "c",
      Path::new("bindings/"),
      None,
      &Default::default(),
      None,
      false,
      None
    ).unwrap();
}