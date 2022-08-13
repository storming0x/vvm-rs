use std::{io, path::PathBuf};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, VyperError>;

/// Error types
#[derive(Debug, Error)]
pub enum VyperError {
    /// Internal error
    // #[error("Vyper Error: {0}")]
    // VyperError(String),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] VyperIoError),
    #[error(transparent)]
    VvmError(#[from] vvm_lib::VyperVmError),

    /// General purpose message
    #[error("{0}")]
    Message(String),
}

impl VyperError {
    pub(crate) fn io(err: io::Error, path: impl Into<PathBuf>) -> Self {
        VyperIoError::new(err, path).into()
    }
    pub fn msg(msg: impl Into<String>) -> Self {
        VyperError::Message(msg.into())
    }
}

#[derive(Debug, Error)]
#[error("\"{}\": {io}", self.path.display())]
pub struct VyperIoError {
    io: io::Error,
    path: PathBuf,
}

impl VyperIoError {
    pub fn new(io: io::Error, path: impl Into<PathBuf>) -> Self {
        Self {
            io,
            path: path.into(),
        }
    }
}

impl From<VyperIoError> for io::Error {
    fn from(err: VyperIoError) -> Self {
        err.io
    }
}
