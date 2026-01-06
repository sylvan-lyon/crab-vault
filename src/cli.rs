mod jwt;
pub mod run;

use clap::{
    builder::{styling, Styles}, ColorChoice, Parser, Subcommand
};

#[derive(Parser)]
#[command(color = ColorChoice::Always)]
#[command(
    styles = Styles::styled()
        .header(styling::AnsiColor::Green.on_default().bold().underline())
        .error(styling::AnsiColor::BrightRed.on_default().bold().underline())
        .usage(styling::AnsiColor::Cyan.on_default().bold().underline())
        .literal(styling::AnsiColor::BrightWhite.on_default().bold())
        .placeholder(styling::AnsiColor::White.on_default().dimmed())
        // .valid(style)
        // .invalid(style)
)]
#[command(version, author, about, long_about = None)]
#[command(disable_help_subcommand = true, subcommand_required = true)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: CliCommand,

    /// Location of configuration file.
    #[arg(long = "config-path", short = 'C')]
    #[arg(default_value = "~/.config/crab-vault/crab-vault.toml")]
    pub config_path: String,
}

impl Cli {
    #[inline(always)]
    pub fn action(&self) -> Action {
        self.subcommand.action()
    }
}

#[derive(Subcommand)]
pub enum CliCommand {
    #[command(about = "Run the server.")]
    #[command(
        long_about = r#"Run the server, but all the attributes passed by cli will override those from config file."#
    )]
    Run(run::RunArgs),

    #[command(subcommand, about = "JWT management commands")]
    Jwt(jwt::Command),
}

/// 这是 [`Cli`] 的简短表现，用于判断将要执行那些操作而不获取对应的值
pub enum Action {
    Run,
    Jwt,
}

impl CliCommand {
    pub const fn action(&self) -> Action {
        match self {
            CliCommand::Run(_) => Action::Run,
            CliCommand::Jwt(_) => Action::Jwt,
        }
    }
}

pub async fn run() {
    let cli = Cli::parse();
    match cli.action() {
        Action::Jwt | Action::Run => {
            let Cli {
                subcommand,
                config_path,
            } = cli;
            exec(subcommand, config_path).await
        }
    }
}

async fn exec(subcommand: CliCommand, config_path: String) {
    match subcommand {
        CliCommand::Jwt(command) => jwt::exec(command, config_path),
        CliCommand::Run(arg) => crate::http::server::run(config_path, arg).await,
    }
}
