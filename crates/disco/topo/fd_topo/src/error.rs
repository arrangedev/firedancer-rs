use firedancer_rs_common::define_errors;

define_errors! {
    TopoError,
    { InvalidName => "Invalid name" },
    { ResourceLimitExceeded => "Resource limit exceeded" },
    { InvalidConfiguration => "Invalid configuration" },
    { MemoryError => "Memory error" },
    { IoError => "I/O error" },
    { SystemError => "System error" },
    { NotFound => "Not found" },
    { Unsupported => "Unsupported" },
}

/// Result type for topology operations.
// pub type TopoResult<T> = Result<T, TopoError>;

// /// Error types that can occur when working with topologies.
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum TopoError {
//     /// Invalid name provided (e.g., too long, contains invalid characters).
//     InvalidName(String),
//     /// Resource limit exceeded (e.g., too many workspaces, tiles, etc.).
//     ResourceLimitExceeded(String),
//     /// Invalid configuration or parameters.
//     InvalidConfiguration(String),
//     /// Memory allocation or management error.
//     MemoryError(String),
//     /// I/O error during workspace operations.
//     IoError(String),
//     /// General system error.
//     SystemError(String),
//     /// Object not found.
//     NotFound(String),
//     /// Operation not supported on current platform.
//     Unsupported(String),
// }

impl From<std::ffi::NulError> for TopoError {
    fn from(err: std::ffi::NulError) -> Self {
        TopoError::InvalidName
    }
}

impl From<std::io::Error> for TopoError {
    fn from(err: std::io::Error) -> Self {
        TopoError::IoError
    }
}
