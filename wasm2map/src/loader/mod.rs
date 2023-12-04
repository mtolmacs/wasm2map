#[cfg(not(feature = "memmap2"))]
pub mod file;
#[cfg(not(feature = "memmap2"))]
pub use file::WasmLoader;

#[cfg(feature = "memmap2")]
pub mod mmap;
#[cfg(feature = "memmap2")]
pub use mmap::WasmLoader;
