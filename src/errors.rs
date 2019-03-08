use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct SmolError(pub exitcode::ExitCode, pub Option<String>);

impl SmolError {
    pub fn from_err<D>(code: exitcode::ExitCode, error: &Error, message: D) -> SmolError
    where
        D: fmt::Display,
    {
        SmolError(code, Some(format!("{}: {}", message, error)))
    }
}

impl From<io::Error> for SmolError {
    fn from(error: io::Error) -> Self {
        SmolError(exitcode::OSFILE, Some(format!("{}", error)))
    }
}

impl From<SmolError> for Result<(), SmolError> {
    fn from(error: SmolError) -> Self {
        Err(error)
    }
}
