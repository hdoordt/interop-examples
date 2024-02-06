#![deny(improper_ctypes_definitions)]

pub mod crc32;

pub mod bsn_diplomat;
pub mod bsn_cbindgen;

#[no_mangle]
pub extern "C" fn say_hello() {
    println!("🦀 Hello, Rusty world! 🦀");
}
