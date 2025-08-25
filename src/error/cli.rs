use std::{
    num::{ParseFloatError, ParseIntError},
    str::ParseBoolError,
};

use clap::{CommandFactory, error::ErrorKind};
use toml_edit::DatetimeParseError;

use crate::cli::Cli;

pub type CliResult<T> = Result<T, CliError>;

pub struct CliError {
    kind: ErrorKind,
    message: String,
}

impl CliError {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Self { kind, message }
    }

    pub fn handle_strait_forward(self) -> ! {
        Cli::command().error(self.kind, self.message).exit()
    }
}

impl From<ParseIntError> for CliError {
    fn from(err: ParseIntError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to an i64 value, details: {err}"),
        )
    }
}

impl From<ParseFloatError> for CliError {
    fn from(err: ParseFloatError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a f64 value, details: {err}"),
        )
    }
}

impl From<ParseBoolError> for CliError {
    fn from(err: ParseBoolError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a bool value, details: {err}"),
        )
    }
}

impl From<DatetimeParseError> for CliError {
    fn from(err: DatetimeParseError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a date time, details: {err}"),
        )
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("io error occurred while reading configuration file, details: {err}"),
        )
    }
}

impl From<toml_edit::TomlError> for CliError {
    fn from(value: toml_edit::TomlError) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("cannot parse the configuration file, details: {value}"),
        )
    }
}
