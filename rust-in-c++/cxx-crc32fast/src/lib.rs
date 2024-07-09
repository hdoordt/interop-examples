#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type Hasher;
        
        fn init() -> Box<Hasher>;
        
        fn update(&mut self, buf: &[u8]);
        
        fn finalize(h: Box<Hasher>) -> u32;
    }
}

struct Hasher(crc32fast::Hasher);

fn init() -> Box<Hasher> {
    Box::new(Hasher::new())
}

fn finalize(h: Box<Hasher>) -> u32 {
    h.finalize()
}

impl Hasher {
    fn new() -> Self {
        Self(crc32fast::Hasher::new())
    }
    
    fn update(&mut self, buf: &[u8]) {
        self.0.update(&buf)
    }

    fn finalize(self) -> u32 {
        self.0.finalize()
    }
}
