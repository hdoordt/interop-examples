# cxx-crc32fast

uses the rust [`crc32fast`](https://docs.rs/crc32fast/latest/crc32fast/index.html) crate from C++

```sh
> cat hello.txt | cargo run
1cf81ca7
> crc32 hello.txt
1cf81ca7
```

## Notes

The `crc32fast` library uses a `Hasher` to keep track of its state, so that input can be fed into it incrementally. Because of limitations in `cxx` this value must be initialized in rust code, and can only be passed to C++ in a `Box`.

> I suppose this can technically be worked around with `alloca`, but compilers don't really like that

Because `cxx` doesn't know anything about the implementation of `crc32fast::Hasher`, we need to explicitly wrap the type and duplicate the parts of its API that we want to use from C++.
