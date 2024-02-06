#![deny(improper_ctypes_definitions)]

mod crc32;

mod bsn;

#[no_mangle]
pub extern "C" fn say_hello() {
    println!("🦀 Hello, Rusty world! 🦀");
}
