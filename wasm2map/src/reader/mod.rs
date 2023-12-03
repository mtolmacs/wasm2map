#[cfg(not(feature = "memmap2"))]
pub mod file;
#[cfg(not(feature = "memmap2"))]
pub use file::WasnReader;

#[cfg(feature = "memmap2")]
pub mod mmap;
#[cfg(feature = "memmap2")]
pub use mmap::WasmReader;
