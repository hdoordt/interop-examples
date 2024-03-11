#![no_main] // main defined in C++ by main.cc

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type Hasher;

        fn init() -> Box<Hasher>;

        fn update(&mut self, buf: &[u8]);

        fn finalize(&self) -> u32;
    }
}

struct Hasher(crc32fast::Hasher);

fn init() -> Box<Hasher> {
    Box::new(Hasher(crc32fast::Hasher::new()))
}

impl Hasher {
    fn update(&mut self, buf: &[u8]) {
        self.0.update(&buf)
    }

    fn finalize(&self) -> u32 {
        self.0.clone().finalize()
    }
}
