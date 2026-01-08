use crate::app_config::{self, AppConfig, ConfigItem};
use crate::error::fatal::FatalError;
use crab_vault::auth::{HttpMethod, Jwt, JwtDecoder, Permission};

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
    #[arg(long)]
    pub nbf_offset: Option<i64>,

    /// Seconds from now when the token becomes invalid (Expiration time). Defaults to 3600 (ttl: 1hr)
    #[arg(long)]
    pub exp_offset: Option<i64>,

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
    pub max_size: Option<usize>,

    /// The allowed content type (UNIX shell wildcard supported) (e.g., application/* or *)
    #[arg(long, value_delimiter = ',', default_value = "*")]
    pub allowed_content_type: Vec<String>,
}

pub fn exec(cmd: Command, config_path: String) {
    let config = app_config::StaticAppConfig::from_file(config_path)
        .into_runtime()
        .map_err(|e| e.exit_now())
        .unwrap();

    match cmd {
        Command::Generate(args) => generate_jwt(args, config),
        Command::Verify => verify_jwt(config),
    }
    .map_err(|e| e.exit_now())
    .unwrap()
}

fn generate_jwt(args: GenerateArgs, config: AppConfig) -> Result<(), FatalError> {
    let jwt_encoder_config = &config.auth.jwt_encoder_config;
    let jwt_encoder = &config.auth.jwt_encoder_config.encoder;

    let iss = if args.issue_as.is_some() {
        args.issue_as.unwrap()
    } else {
        jwt_encoder_config.issue_as.to_string()
    };

    let aud = if args.audiences.is_some() {
        args.audiences.unwrap()
    } else {
        jwt_encoder_config.audience.to_vec()
    };

    let payload = Permission::new_minimum()
        .permit_method(args.operations)
        .permit_resource_pattern(args.resource_pattern)
        .restrict_maximum_size_option(args.max_size)
        .permit_content_type(args.allowed_content_type);

    let claims = Jwt::new(iss, &aud, payload)
        .expires_in(Duration::seconds(
            args.exp_offset
                .unwrap_or(config.auth.jwt_encoder_config.expires_in.num_seconds()),
        ))
        .not_valid_in(Duration::seconds(
            args.nbf_offset
                .unwrap_or(config.auth.jwt_encoder_config.not_valid_in.num_seconds()),
        ));

    // 编码 JWT
    let token = jwt_encoder
        .encode_randomly(&claims)
        .map_err(|e| FatalError::new(ErrorKind::Io, format!("JWT encoding failed: {e}"), None))?;

    println!("{}", token);
    Ok(())
}

fn verify_jwt(config: AppConfig) -> Result<(), FatalError> {
    let mut token = String::new();
    io::stdin().read_to_string(&mut token).map_err(|e| {
        FatalError::new(
            ErrorKind::Io,
            format!("Nothing to read from standard input as token input: {e}"),
            None,
        )
    })?;

    let token = token.trim();
    if token.is_empty() {
        return Err(FatalError::new(
            ErrorKind::Io,
            "No token received from standard input.".to_string(),
            None,
        ));
    }

    let jwt_decoder = config.auth.jwt_decoder_config.decoder;

    // 解码
    let decoded = JwtDecoder::decode_unchecked(token).map_err(FatalError::from)?;
    let pretty_json = serde_json::to_string_pretty(&decoded).map_err(FatalError::from)?;

    // 验证
    match jwt_decoder.decode::<Permission>(token) {
        Ok(_) => eprintln!("Token verified successfully. Payload (Claims):\n"),
        Err(e) => eprintln!("Token invalid because of {e}. Payload (Claims):\n"),
    }

    println!("{}", pretty_json);
    Ok(())
}
