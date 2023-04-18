use std::fmt::Display;

/// Common error type for the crate
#[derive(Debug)]
pub struct Error {
    msg: String,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error {
            msg: value.to_string(),
        }
    }
}

impl From<object::Error> for Error {
    fn from(value: object::Error) -> Self {
        Error {
            msg: value.to_string(),
        }
    }
}

impl From<gimli::Error> for Error {
    fn from(value: gimli::Error) -> Self {
        Error {
            msg: value.to_string(),
        }
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error {
            msg: value.to_owned(),
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error {
            msg: value,
        }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(value: std::num::TryFromIntError) -> Self {
        Error {
            msg: value.to_string(),
        }
    }
}
