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
    /// Seconds from now when the token becomes valid (Not Before). Defaults to 0 (valid immediately)
    #[arg(long, default_value_t = 0)]
    pub nbf_offset: i64,

    /// Seconds from now when the token becomes invalid (Expiration time). Defaults to 3600 (ttl: 1hr)
    #[arg(long, default_value_t = 3600)]
    pub exp_offset: i64,

    /// The issuer of this token (if set), if not provided, we'll randomly select one issuer from your configuration file, or make it `null`
    #[arg(long)]
    pub issuer: Option<String>,

    /// The audiences of this token (if set), if not provided, we'll fetch audiences from your configuration file, default value of configuration file is an empty array
    #[arg(long, value_delimiter = ',')]
    pub audiences: Option<Vec<String>>,

    /// Allowed HTTP operations, comma-separated (e.g., get,POST)
    #[arg(long, value_delimiter = ',', default_value = "all")]
    pub operations: Vec<HttpMethod>,

    /// Resource path pattern for this token (e.g., "/data/*")
    #[arg(long, default_value = "*")]
    pub resource_pattern: String,

    /// The max size of a request body (in bytes), if not provided, the http request body can be extremely giant (MAX to u64)
    #[arg(long)]
    pub max_size: Option<u64>,

    /// The allowed content type (UNIX shell wildcard supported) (e.g., application/* or *)
    #[arg(long, value_delimiter = ',', default_value = "*")]
    pub allowed_content_type: Vec<String>,
}

pub fn exec(cmd: Command) {
    match cmd {
        Command::Generate(args) => generate_jwt(args),
        Command::Verify => verify_jwt(),
    }
    .map_err(|e| e.exit_now())
    .unwrap()
}

fn generate_jwt(args: GenerateArgs) -> Result<(), CliError> {
    let jwt_config = app_config::server()
        .auth()
        .jwt_config_builder()
        .clone()
        .build()
        .map_err(|e| e.exit_now())
        .unwrap();
    let validation_config = &jwt_config.validation;

    let iss = if args.issuer.is_some() {
        args.issuer
    } else {
        validation_config.iss.as_ref().and_then(|issuers| {
            let issuers_vec: Vec<_> = issuers.iter().collect();
            issuers_vec
                .get(rand::random_range(0..issuers_vec.len()))
                .map(|s| (*s).clone())
        })
    };

    let aud = if args.audiences.is_some() {
        args.audiences.unwrap()
    } else {
        validation_config
            .aud
            .as_ref()
            .map(|aud| aud.iter().cloned().collect())
            .unwrap_or_default()
    };

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
            conditions: Conditions {
                max_size: args.max_size,
                allowed_content_types: args.allowed_content_type,
            },
        },
    };

    // 编码 JWT
    let token = Jwt::encode(&claims, &jwt_config)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("JWT encoding failed: {e}"), None))?;

    println!("{}", token);
    Ok(())
}

fn verify_jwt() -> Result<(), CliError> {
    let mut token = String::new();
    io::stdin().read_to_string(&mut token).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("Nothing to read from standard input as token input: {e}"),
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

    let jwt_config = app_config::server()
        .auth()
        .jwt_config_builder()
        .clone()
        .build()
        .map_err(|e| e.exit_now())
        .unwrap();

    // 解码
    let decoded = Jwt::<Permission>::decode_unchecked(token, &jwt_config).map_err(CliError::from)?;
    let pretty_json = serde_json::to_string_pretty(&decoded).map_err(CliError::from)?;

    // 验证
    match Jwt::<Permission>::decode(token, &jwt_config) {
        Ok(_) => println!("Token verified successfully. Payload (Claims):\n"),
        Err(e) => println!("Token invalid because of {e}. Payload (Claims):\n"),
    }

    println!("{}", pretty_json);
    Ok(())
}
