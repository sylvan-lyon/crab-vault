use crate::app_config;
use crate::error::cli::CliError;
use crab_vault_auth::{HttpMethod, Jwt, Permission};

use chrono::Duration;
use clap::error::ErrorKind;
use clap::{Args, Subcommand};
use std::io::{self, Read};

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
    pub issue_as: Option<String>,

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

    let iss = if args.issue_as.is_some() {
        args.issue_as
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

    let payload = Permission::new_minimum()
        .permit_method(args.operations)
        .permit_access_url(args.resource_pattern)
        .restrict_maximum_size_option(args.max_size)
        .permit_content_type(args.allowed_content_type);

    let claims = Jwt::new(payload)
        .issue_as_option(iss.as_ref())
        .audiences(&aud)
        .expires_in(Duration::seconds(args.exp_offset))
        .not_valid_in(Duration::seconds(args.nbf_offset));

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
    let decoded = Jwt::<Permission>::decode_unchecked(token).map_err(CliError::from)?;
    let pretty_json = serde_json::to_string_pretty(&decoded).map_err(CliError::from)?;

    // 验证
    match Jwt::<Permission>::decode(token, &jwt_config) {
        Ok(_) => println!("Token verified successfully. Payload (Claims):\n"),
        Err(e) => println!("Token invalid because of {e}. Payload (Claims):\n"),
    }

    println!("{}", pretty_json);
    Ok(())
}
