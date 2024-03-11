#![no_main] // main defined in C++ by main.cc

use cxx::CxxString;
use std::io::{self, Write};
use std::pin::Pin;

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        fn prettify_json(input: &[u8], output: Pin<&mut CxxString>) -> Result<()>;
    }
}

struct WriteToCxxString<'a>(Pin<&'a mut CxxString>);

impl<'a> Write for WriteToCxxString<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.as_mut().push_bytes(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn prettify_json(input: &[u8], output: Pin<&mut CxxString>) -> serde_json::Result<()> {
    let writer = WriteToCxxString(output);
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let mut serializer = serde_json::Serializer::pretty(writer);
    serde_transcode::transcode(&mut deserializer, &mut serializer)
}
