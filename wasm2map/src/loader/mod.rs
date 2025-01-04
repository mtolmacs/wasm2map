/// Helper module to load wasm a file into memory easily. Currently loading from
/// a file is supported.
///
/// The type of 'Loader' is determined by the feature flag 'memmap2'. If the
/// memmap2 feature is enabled, then the loader uses unsafe code to use the
/// fast mmap OS feature to load the file into memory in one swoop. Otherwise,
/// if safe code is required, then the file loader uses traditional file I/O
/// to do the same.

#[cfg(not(feature = "memmap2"))]
pub mod file;
#[cfg(not(feature = "memmap2"))]
pub use file::Loader;

#[cfg(feature = "memmap2")]
pub mod mmap;
#[cfg(feature = "memmap2")]
pub use mmap::Loader;
