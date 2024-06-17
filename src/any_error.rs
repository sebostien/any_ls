use std::error;
use std::fmt;

#[derive(Debug)]
pub enum AnyError {
    NotYetImplemented(&'static str),
}

impl error::Error for AnyError {}

impl fmt::Display for AnyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotYetImplemented(msg) => {
                write!(f, "Not yet implemented: {msg}")
            }
        }
    }
}
