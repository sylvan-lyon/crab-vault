use std::{
    num::{ParseFloatError, ParseIntError},
    str::ParseBoolError,
};

use clap::{CommandFactory, error::ErrorKind};
use crab_vault_auth::error::AuthError;
use toml_edit::DatetimeParseError;

use crate::cli::Cli;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub struct CliError {
    kind: ErrorKind,
    general_message: String,
    source: Vec<String>,
}

#[derive(Default)]
pub struct MultiCliError {
    errors: Vec<CliError>,
}

impl MultiCliError {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    pub fn add(&mut self, error: CliError) -> &mut Self {
        self.errors.push(error);
        self
    }

    pub fn exit_now(self) -> ! {
        let mut final_message = "".to_string();
        for e in self.errors {
            final_message.push_str(&format!("\n\n{}", e.into_message()));
        }

        Cli::command().error(ErrorKind::Io, final_message).exit()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.len() == 0
    }
}

impl CliError {
    pub fn new(kind: ErrorKind, general_message: String, source: Option<String>) -> Self {
        Self {
            kind,
            general_message,
            source: match source {
                Some(val) => vec![val],
                None => vec![],
            },
        }
    }

    pub fn exit_now(self) -> ! {
        let (kind, message) = (self.kind, self.into_message());
        Cli::command()
            .error(kind, format!("\n\n    {message}"))
            .exit()
    }

    pub fn add_source(mut self, source: String) -> Self {
        self.source.push(source);
        self
    }

    pub fn into_message(self) -> String {
        if self.source.is_empty() {
            format!("    - {}", self.general_message)
        } else {
            let mut message = format!("    - {}", self.general_message);
            for src in self.source.into_iter().rev() {
                message.push_str(&format!("\n    | {src}"))
            }
            message
        }
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
        Self::new(ErrorKind::Io, format!("base64 error: {}", value), None)
    }
}

impl From<AuthError> for CliError {
    fn from(value: AuthError) -> Self {
        let (general_message, source) = match value {
            AuthError::MissingAuthHeader => ("missing auth header".into(), None),
            AuthError::InvalidAuthFormat => ("invalid token format".into(), None),
            AuthError::InvalidKeyId => ("invalid key id".into(), None),
            AuthError::InvalidToken => ("token is invalid".into(), None),
            AuthError::TokenExpired => ("token expired".into(), None),
            AuthError::TokenNotYetValid => ("token not yet valid".into(), None),
            AuthError::InvalidSignature => ("token signature is invalid".into(), None),
            AuthError::InvalidAlgorithm(alg) => {
                (format!("cannot validate token encoded by {:?}", alg), None)
            }
            AuthError::InvalidIssuer => ("token is issued by untrusted issuer".into(), None),
            AuthError::InvalidAudience => ("token has invalid audience".into(), None),
            AuthError::InvalidSubject => ("subject of this token is invalid".into(), None),
            AuthError::MissingClaim(claim) => (format!("claim `{claim}` is absent"), None),
            AuthError::InsufficientPermissions => ("the permission is not sufficient".into(), None),
            AuthError::TokenRevoked => ("this token is revoked by the server".into(), None),
            AuthError::InvalidUtf8(e) => (
                format!("the token has some invalid utf-8 character, details: {e}"),
                None,
            ),
            AuthError::InvalidJson(e) => (
                format!("this token cannot be deserialized, details: {e}"),
                None,
            ),
            AuthError::InvalidBase64(e) => (
                format!("this token is not encoded in standard base64, details: {e}"),
                None,
            ),
            AuthError::InternalError(e) => (
                format!("something wrong while handling the token, details: {e}"),
                None,
            ),
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
