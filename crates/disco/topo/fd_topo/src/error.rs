use firedancer_rs_common::define_errors;

define_errors! {
    TopoError,
    { InvalidInput => "Invalid input" },
    { InvalidName => "Invalid name" },
    { ResourceLimitExceeded => "Resource limit exceeded" },
    { InvalidConfiguration => "Invalid configuration" },
    { MemoryError => "Memory error" },
    { IoError => "I/O error" },
    { SystemError => "System error" },
    { NotFound => "Not found" },
    { Unsupported => "Unsupported" },
}

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
