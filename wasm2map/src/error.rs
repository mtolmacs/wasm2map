use std::{fmt::Debug, io, num::TryFromIntError};

/// Common public error type for the library which is exported from the crate
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    /// Signals an issue with the WASM object file structure, segments or reading
    Object(#[from] object::Error),
    /// Signals an issue with the DWARF data structures in the object file
    /// or parsing of the DWARF data
    Dwarf(#[from] gimli::Error),
    /// When the source file or output file cannot be read or mapped to memory
    IoError(#[from] io::Error),
    /// Internal error which shouldn't ever happen. Signals a programming error
    /// with this lib or the downstream dependencies, but panicking in libraries
    /// is not nice with the upstream implementor, so we wrap it up with this.
    Internal(#[from] InternalError),
}

/// The opaque internal error type for programming errors. Should not be exposed
/// outside the library.
#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("Internal Error: {0}")]
    Generic(String),
    #[error("Internal Error: {0}")]
    TryFromInt(#[from] TryFromIntError),
}

impl From<String> for InternalError {
    fn from(value: String) -> Self {
        Self::Generic(value)
    }
}

impl From<&'static str> for InternalError {
    fn from(value: &'static str) -> Self {
        Self::Generic(String::from(value))
    }
}
