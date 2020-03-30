# rust-shed

rust-shed is a repository containing Rust crates common between other Facebook
open source projects (like Mononoke or Eden).

## Building rust-shed

### TL;DR

You can use `cargo` to build and test the project.

When using `thrift_compiler` you have to have fbthrfit compiler installed.
For MacOS/Unix to install it inside `$HOME/build` do:
```
[rust-shed]$ mkdir $HOME/build
[rust-shed]$ ./build/fbcode_builder/getdeps.py build fbthrift --install-prefix $HOME/build
```
After that add `THRIFT=$HOME/build/fbthrift/bin/thrift1` to your environment or make sure
`thrift1` is accessible by adding `$HOME/build/fbthrift/bin` to `PATH`.

Alternatively you can build and run tests with:
```
[rust-shed]$ ./build/fbcode_builder/getdeps.py build rust-shed
[rust-shed]$ ./build/fbcode_builder/getdeps.py test rust-shed
```

### Dependencies

- [Cargo](https://github.com/rust-lang/cargo) is used for building and testing
- The `thrift_compiler` crate requires
  [fbthrift](https://github.com/facebook/fbthrift) to be installed or the
  `THRIFT` environment variable to point to the thrift compiler

## Contributing

See the [CONTRIBUTING](CONTRIBUTING.md) file for how to help out.

## License

rust-shed is both MIT and Apache License, Version 2.0 licensed, as found
in the [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) files.
