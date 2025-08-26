use std::{
    num::{ParseFloatError, ParseIntError},
    str::ParseBoolError,
};

use clap::{CommandFactory, error::ErrorKind};
use toml_edit::DatetimeParseError;

use crate::{cli::Cli, error::auth::AuthError};

pub type CliResult<T> = Result<T, CliError>;

pub struct CliError {
    kind: ErrorKind,
    general_message: String,
    source: Option<String>,
}

impl CliError {
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }
}

impl CliError {
    pub fn new(kind: ErrorKind, general_message: String, source: Option<String>) -> Self {
        Self {
            kind,
            general_message,
            source,
        }
    }

    pub fn handle_strait_forward(self) -> ! {
        let message;
        if let Some(src) = self.source {
            message = format!("\n{}\n    {src}", self.general_message);
        } else {
            message = self.general_message;
        }
        Cli::command().error(self.kind, message).exit()
    }
}

impl From<ParseIntError> for CliError {
    fn from(err: ParseIntError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to an i64 value, details: {err}"),
            None,
        )
    }
}

impl From<ParseFloatError> for CliError {
    fn from(err: ParseFloatError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a f64 value, details: {err}"),
            None,
        )
    }
}

impl From<ParseBoolError> for CliError {
    fn from(err: ParseBoolError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a bool value, details: {err}"),
            None,
        )
    }
}

impl From<DatetimeParseError> for CliError {
    fn from(err: DatetimeParseError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a date time, details: {err}"),
            None,
        )
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("io error occurred while reading configuration file, details: {err}"),
            None,
        )
    }
}

impl From<toml_edit::TomlError> for CliError {
    fn from(value: toml_edit::TomlError) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("cannot parse the configuration file, details: {value}"),
            None,
        )
    }
}

impl From<base64::DecodeError> for CliError {
    fn from(value: base64::DecodeError) -> Self {
        match value {
            base64::DecodeError::InvalidByte(offset, byte) => Self::new(
                ErrorKind::Io,
                format!(
                    "Invalid byte while handling the base64 input{offset}{:0x}",
                    byte
                ),
                None,
            ),
            base64::DecodeError::InvalidLength(_) => todo!(),
            base64::DecodeError::InvalidLastSymbol(_, _) => todo!(),
            base64::DecodeError::InvalidPadding => todo!(),
        }
    }
}

impl From<AuthError> for CliError {
    fn from(value: AuthError) -> Self {
        let (general_message, source) = match value {
            AuthError::MissingAuthHeader => ("missing auth header".to_string(), None),
            AuthError::InvalidAuthFormat => ("invalid token format".to_string(), None),
            AuthError::TokenInvalid => ("token is invalid".to_string(), None),
            AuthError::TokenExpired => ("token expired".to_string(), None),
            AuthError::TokenNotYetValid => ("token not yet valid".to_string(), None),
            AuthError::InvalidSignature => ("token signature is invalid".to_string(), None),
            AuthError::InvalidIssuer => ("token is issued by untrusted issuer".to_string(), None),
            AuthError::InvalidAudience => ("token has invalid audience".to_string(), None),
            AuthError::InvalidSubject => ("subject of this token is invalid".to_string(), None),
            AuthError::MissingClaim(claim) => (format!("claim {claim} is absent"), None),
            AuthError::InsufficientPermissions => ("the permission is not sufficient".to_string(), None),
            AuthError::TokenRevoked => ("this token is revoked by the server".to_string(), None),
            AuthError::InternalError(e) => ("something wrong while handling the token".to_string(), Some(e)),
        };

        Self::new(ErrorKind::Io, general_message, source)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        match value.classify() {
            serde_json::error::Category::Io => todo!(),
            serde_json::error::Category::Syntax => todo!(),
            serde_json::error::Category::Data => todo!(),
            serde_json::error::Category::Eof => todo!(),
        }
    }
}
