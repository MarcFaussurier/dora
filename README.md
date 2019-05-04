New syntax: 

Assignement use the := token instead of =
```
class StringBuf(var buf: Array<Byte>, var length: Int) {
  static fun empty() -> StringBuf := StringBuf(Array::<Byte>(0), 0); 
 ... 
 ```
Also the comparaison is = instead of == and the strict one is == instead of === 
  
 Fibonacci sample: 
 ```
 fun fibo(n: Int) -> Int {
    if n =  0 {
        return n;
    } else if n <= 2 {
        return 1;
    } else {
        return fibo(n - 1) + fibo(n - 2);
    }
}
```

# Dora

[![Join the chat at https://gitter.im/dora-lang/dora](https://badges.gitter.im/dora-lang/Lobby.svg)](https://gitter.im/dora-lang/Lobby?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge) [![Build Status](https://travis-ci.org/dinfuehr/dora.svg?branch=master)](https://travis-ci.org/dinfuehr/dora)

JIT-compiler for the programming language Dora implemented in Rust.
Works on Linux (x86\_64, aarch64) and macOS (x86\_64).
Build with:

## Dependencies
You need to install these dependencies:

```
# on Fedora
$ sudo dnf install capstone-devel ruby

# on Ubuntu/Debian
$ sudo apt install libcapstone-dev ruby

# on MacOS capstone can be installed via homebrew
$ brew install capstone
```

[Ruby](https://www.ruby-lang.org/) is used for running tests, while [capstone](https://github.com/aquynh/capstone) is used for instruction decoding/disassembling machine code.


## Compilation & Testing
Install current Rust Nightly via [rustup.rs](http://rustup.rs). The nightly version of
Rust is needed because Dora uses some unstable features of Rust (e.g. inline assembly).

Dora uses [cargo](http://crates.io) for building, which is bundled with Rust:

```
# install last nightly and use it for this project
rustup update nightly
rustup override set nightly

# run all tests in debug and release mode
tools/test
tools/test-release
```
