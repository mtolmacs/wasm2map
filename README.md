# Cargo WASM Sourcemap Utility

![Build status](https://github.com/mtolmacs/rust-wasm2map/actions/workflows/main.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/wasm2map.svg)](https://crates.io/crates/wasm2map)
[![Documentation](https://docs.rs/wasm2map/badge.svg)](https://docs.rs/wasm2map)

Generates a browser-supported sourcemap for WASM binaries containing DWARF debug information and associates it with the WASM binary, so when loaded in the browser you can see the rust line, character and source code (if available) in the debug panel and console.

# Usage

1. Use it with Cargo to manually preprocess your WASM binary before serving:

```
 % cargo install cargo-wasm2map

 # Build your WASM binary the way you usually do
 % cargo build --target wasm32-unknown-unknown

 % cargo wasm2map

 # Run the WASM in your browser...
```

2. Use it as a library in your utility:

```
extern crate wasm2map;

use wasm2map::*;
```

# Current limitations
- It does not load the std source code, so those files will not be loaded (but file name, line and character position is dumped in the console on panic)

# Contribution
Your contributions are welcome, especially bug reports and testing on various platforms. Feel free to open a PR if you can contribute a fix.

If you would like to contribute an API change, extension or a new trait implementation, please open an issue first and discuss before starting work on a PR. For details please read the CONTRIBUTING.md file.

# License
Licensed under either of Apache License, Version 2.0 or MIT license at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

`SPDX-License-Identifier: Apache-2.0 OR MIT`