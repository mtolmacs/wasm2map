# Cargo WASM Sourcemap Utility

![Build status](https://github.com/mtolmacs/wasm2map/actions/workflows/test.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/wasm2map.svg)](https://crates.io/crates/wasm2map)
[![Documentation](https://docs.rs/wasm2map/badge.svg)](https://docs.rs/wasm2map)
![Min Rust 1.64.0](https://badgen.net/badge/Min%20Rust/1.64.0)

Generates a browser-supported sourcemap for WASM binaries containing DWARF debug information and associates it with the WASM binary, so when loaded in the browser you can see the rust line, character and source code (if available) in the debug panel and console.

NOTE: Can build without unsafe code (the only unsafe code is related to using the memmap2 crate).

### Before
![Before WASM sourcemapping](https://raw.githubusercontent.com/mtolmacs/wasm2map/main/assets/before.png)

### After
![After WASM sourcemapping](https://raw.githubusercontent.com/mtolmacs/wasm2map/main/assets/after.png)

# Usage

1. Use it with Cargo to manually preprocess your WASM binary before serving:

```sh
 cargo install cargo-wasm2map

 # Build your WASM binary the way you usually do
 cargo build --target wasm32-unknown-unknown

# Generate sourcemap for the target WASM (replace myproject with your project
# name).
 cargo wasm2map target/wasm32-unknown-unknown/debug/myproject.wasm \
    -patch -base-url http://localhost:8080

 # Serve the WASM to your browser... (i.e. http://localhost:8080 or wherever
 # your index.html is)
```

2. Use it as a library in your utility:

```rust
use wasm2map::WASM;

let mapper = WASM::load("/path/to/the/file.wasm");
    if let Ok(mut mapper) = mapper {
        let sourcemap = mapper.map_v3(false);
        mapper.patch("http://localhost:8080").expect("Failed to patch");
}
```

# Current limitations
- It does not bundle the source code in the sourcemap currently, so source browsing will not work in the browser

# Contribution
Your contributions are welcome, especially bug reports and testing on various platforms. Feel free to open a PR if you can contribute a fix.

If you would like to contribute an API change, extension or a new trait implementation, please open an issue first and discuss before starting work on a PR. For details please read the CONTRIBUTING.md file.

# License
Licensed under either of Apache License, Version 2.0 or MIT license at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

`SPDX-License-Identifier: Apache-2.0 OR MIT`