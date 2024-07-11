# cxx-pretty-json

adapted from [https://cxx.rs/binding/slice.html](https://cxx.rs/binding/slice.html).

uses the rust [`serde_json`](https://github.com/serde-rs/json) and [`serde_transcode`](https://github.com/sfackler/serde-transcode) to pretty print json from C++.

```sh
> echo '{"fearless":"concurrency"}' | cargo run -q
{
  "fearless": "concurrency"
}
```

## Setup

The rust code in `main.rs` exposes one function to C++: `prettify_json`. The `CxxString` type is used for zero-cost transfer of data from Rust back to C++.

The C++ code reads input from stdin (but could aquire its input from anywhere) and passes the data and an allocation for the output to rust.
This is nice, and the code required for the interoperation between Rust and C++ is minimal.
