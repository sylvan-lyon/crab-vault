use std::{
    num::{ParseFloatError, ParseIntError},
    str::ParseBoolError,
};

use clap::{CommandFactory, error::ErrorKind};
use crab_vault::auth::error::AuthError;
use toml_edit::DatetimeParseError;

use crate::cli::Cli;

pub type FatalResult<T> = Result<T, MultiFatalError>;

#[derive(Debug)]
pub struct FatalError {
    kind: ErrorKind,
    general_message: String,
    when: Vec<String>,
}

#[derive(Default, Debug)]
pub struct MultiFatalError {
    errors: Vec<FatalError>,
}

impl MultiFatalError {
    #[inline]
    pub const fn new() -> Self {
        Self { errors: vec![] }
    }

    #[inline]
    pub fn push(&mut self, error: FatalError) {
        self.errors.push(error);
    }

    #[inline]
    pub fn append(&mut self, rhs: &mut MultiFatalError) -> &mut Self {
        self.errors.append(&mut rhs.errors);
        self
    }

    pub fn exit_now(self) -> ! {
        let mut final_message = "".to_string();
        for e in self.errors {
            final_message.push_str(&format!("\n\n{}", e.into_message()));
        }

        Cli::command().error(ErrorKind::Io, final_message).exit()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl FatalError {
    pub fn new(kind: ErrorKind, general_message: String, when: Option<String>) -> Self {
        Self {
            kind,
            general_message,
            when: match when {
                Some(val) => vec![val],
                None => vec![],
            },
        }
    }

    pub fn exit_now(self) -> ! {
        let (kind, message) = (self.kind, self.into_message());
        Cli::command()
            .error(kind, format!("\n\n{message}"))
            .exit()
    }

    pub fn when(mut self, source: String) -> Self {
        self.when.push(source);
        self
    }

    pub fn into_message(self) -> String {
        if self.when.is_empty() {
            format!("    * {}", self.general_message)
        } else {
            let mut message = format!("    * {}", self.general_message);
            for src in self.when.into_iter().rev() {
                message.push_str(&format!("\n      {src}"))
            }
            message
        }
    }
}

impl From<ParseIntError> for FatalError {
    fn from(err: ParseIntError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to an i64 value, details: {err}"),
            None,
        )
    }
}

impl From<ParseFloatError> for FatalError {
    fn from(err: ParseFloatError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a f64 value, details: {err}"),
            None,
        )
    }
}

impl From<ParseBoolError> for FatalError {
    fn from(err: ParseBoolError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a bool value, details: {err}"),
            None,
        )
    }
}

impl From<DatetimeParseError> for FatalError {
    fn from(err: DatetimeParseError) -> Self {
        Self::new(
            ErrorKind::InvalidValue,
            format!("cannot transfer the value to a date time, details: {err}"),
            None,
        )
    }
}

impl From<std::io::Error> for FatalError {
    fn from(err: std::io::Error) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("io error occurred while reading configuration file, details: {err}"),
            None,
        )
    }
}

impl From<toml_edit::TomlError> for FatalError {
    fn from(value: toml_edit::TomlError) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("cannot parse the configuration file, details: {value}"),
            None,
        )
    }
}

impl From<base64::DecodeError> for FatalError {
    fn from(value: base64::DecodeError) -> Self {
        Self::new(ErrorKind::Io, format!("base64 error: {}", value), None)
    }
}

impl From<AuthError> for FatalError {
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

impl From<serde_json::Error> for FatalError {
    fn from(value: serde_json::Error) -> Self {
        match value.classify() {
            serde_json::error::Category::Io => todo!(),
            serde_json::error::Category::Syntax => todo!(),
            serde_json::error::Category::Data => todo!(),
            serde_json::error::Category::Eof => todo!(),
        }
    }
}

impl From<glob::PatternError> for FatalError {
    fn from(e: glob::PatternError) -> Self {
        Self::new(
            ErrorKind::Io,
            format!("pattern incorrect, because {} at {}", e.msg, e.pos),
            None,
        )
    }
}
