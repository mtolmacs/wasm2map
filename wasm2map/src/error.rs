use std::{
    fmt::{Debug, Display},
    num::TryFromIntError,
};

/// Common public error type for the library which is exported from the crate
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    /// Signals an issue with the WASM object file structure, segments or reading
    Object {
        #[from]
        source: object::Error,
    },
    /// Signals an issue with the DWARF data structures in the object file
    /// or parsing of the DWARF data
    Dwarf {
        #[from]
        source: gimli::Error,
    },
    /// Internal error which shouldn't ever happen. Signals a programming error
    /// with this lib or the downstream dependencies, but panicking in libraries
    /// is not nice with the upstream implementor, so we wrap it up with this.
    Internal {
        #[from]
        source: InternalError,
    },
}

/// The opaque internal error type for programming errors. Should not be exposed
/// outside the library.
#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    Generic(&'static str),
    TryFromInt(#[from] TryFromIntError),
}

impl Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
