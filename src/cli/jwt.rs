use crate::app_config;
use crate::error::cli::CliError;
use crate::http::auth::{Conditions, HttpMethod, Jwt, Permission};

use chrono::Utc;
use clap::error::ErrorKind;
use clap::{Args, Subcommand};
use std::io::{self, Read};
use uuid::Uuid;

#[derive(Args)]
pub struct JwtCommandAndArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    /// Generate a new JWT based on the configuration file
    #[command(name = "generate")]
    Generate(GenerateArgs),
    /// Verify a JWT from standard input and print its payload
    #[command(name = "verify")]
    Verify,
}

/// 'generate' 命令的参数
#[derive(Args, Clone)]
pub struct GenerateArgs {
    /// Seconds from now when the token becomes valid (Not Before). Defaults to 0 (valid immediately).
    #[arg(long, default_value_t = 0)]
    pub nbf_offset: i64,

    /// Seconds from now when the token becomes invalid (Expiration time). Defaults to 3600 (ttl: 1hr)
    #[arg(long, default_value_t = 3600)]
    pub exp_offset: i64,

    /// Allowed HTTP operations, comma-separated (e.g., get,POST)
    #[arg(long, value_delimiter = ',', default_value = "all")]
    pub operations: Vec<HttpMethod>,

    /// Resource path pattern for this token (e.g., "/data/*")
    #[arg(long, default_value = "*")]
    pub resource_pattern: String,
}

pub fn exec(cmd: Command) {
    match cmd {
        Command::Generate(args) => generate_jwt(args),
        Command::Verify => verify_jwt(),
    }
    .map_err(|e| e.handle_strait_forward())
    .unwrap()
}

fn generate_jwt(args: GenerateArgs) -> Result<(), CliError> {
    let jwt_config = app_config::server().auth().jwt_config();
    let validation_config = &jwt_config.validation;

    let iss = validation_config.iss.as_ref().and_then(|issuers| {
        let issuers_vec: Vec<_> = issuers.iter().collect();
        issuers_vec
            .get(rand::random_range(0..issuers_vec.len()))
            .map(|s| (*s).clone())
    });

    let aud = validation_config
        .aud
        .as_ref()
        .map(|aud| aud.iter().cloned().collect())
        .unwrap_or(vec![]);

    let iat = Utc::now().timestamp();
    let nbf = iat + args.nbf_offset;
    let exp = iat + args.exp_offset;

    let claims = Jwt {
        iss,
        aud,
        exp,
        nbf,
        iat,
        jti: Uuid::new_v4().as_u128(),
        payload: Permission {
            operations: args.operations,
            resource_pattern: args.resource_pattern,
            conditions: Conditions::default(),
        },
    };

    // 编码 JWT
    let token = Jwt::encode(&claims, jwt_config)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("JWT encoding failed: {e}"), None))?;

    println!("{}", token);
    Ok(())
}

fn verify_jwt() -> Result<(), CliError> {
    let mut token = String::new();
    io::stdin().read_to_string(&mut token).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("Failed to read token from standard input: {e}"),
            None,
        )
    })?;

    let token = token.trim();
    if token.is_empty() {
        return Err(CliError::new(
            ErrorKind::Io,
            "No token received from standard input.".to_string(),
            None,
        ));
    }

    let jwt_config = app_config::server().auth().jwt_config();

    // 解码并验证 token
    let decoded = Jwt::<Permission>::decode(token, jwt_config).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("Token verification failed: {e}"),
            None,
        )
    })?;

    // 将解码后的载荷美化为 JSON 字符串
    let pretty_json = serde_json::to_string_pretty(&decoded).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("Failed to serialize decoded token: {e}"),
            None,
        )
    })?;

    println!("Token verified successfully. Payload (Claims):\n");

    println!("{}", pretty_json);
    Ok(())
}
