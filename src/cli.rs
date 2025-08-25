use clap::{
    Args, Parser, Subcommand,
    builder::{Styles, styling},
};

#[derive(Clone, Parser)]
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
    #[arg(default_value = "crab-vault.toml")]
    pub config_path: String,
}

impl Cli {
    #[inline(always)]
    pub fn action(&self) -> Action {
        self.subcommand.action()
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum CliCommand {
    #[command(about = "Run the server.")]
    #[command(
        long_about = r#"Run the server, but all the attributes passed by cli will override those from config file."#
    )]
    Run {
        /// Listening port number of server.
        #[arg(long = "port", short = 'p')]
        port: Option<u16>,

        /// Sepcify the source of `data`.
        #[arg(long = "data-source", short = None)]
        data_source: Option<String>,

        /// Specify the source of `meta`.
        #[arg(long = "meta-source", short = None)]
        meta_source: Option<String>,

        /// Minimum log level of server.
        #[arg(long = "log-level", short = 'L')]
        log_level: Option<String>,

        /// Log file dump path, or no log file will be saved
        #[arg(long = "dump-path", short = None)]
        dump_path: Option<String>,
    },

    #[command(about = "Set / Unset / Show the configuration item(s).", long_about = None)]
    Config(config::Args),
}

/// 这是 [`Cli`] 的简短表现，用于判断将要执行那些操作而不获取对应的值
pub enum Action {
    Run,
    Config,
}

impl CliCommand {
    pub const fn action(&self) -> Action {
        match self {
            CliCommand::Run {
                port: _,
                data_source: _,
                meta_source: _,
                log_level: _,
                dump_path: _,
            } => Action::Run,
            CliCommand::Config(_) => Action::Config,
        }
    }
}

pub mod config {
    use super::*;

    #[derive(Args, Clone, Debug)]
    #[command(args_conflicts_with_subcommands = true)]
    #[command(flatten_help = true)]
    pub struct Args {
        #[command(subcommand)]
        pub command: ConfigSubcommand,
    }

    #[derive(Subcommand, Clone, Debug)]
    pub enum ConfigSubcommand {
        Set {
            #[arg(help = "Which field to be set")]
            field_path: String,

            #[arg(help = "Value to the specified field")]
            value: String,
        },
        Show {
            #[arg(
                help = "Which field/section to be shown, if missing, the whole config will be shown in toml form"
            )]
            field_path: Option<String>,
        },
        Unset {
            #[arg(help = "Which field/section to be unset")]
            field_path: String,
        },
    }
}

pub async fn run() {
    let cli = Cli::parse();
    match cli.action() {
        Action::Config => exec(cli).await,
        Action::Run => unreachable!(),
    }
}

async fn exec(cli: Cli) {
    use config::*;
    let Cli {
        subcommand,
        config_path,
    } = cli;
    if let CliCommand::Config(config::Args { command }) = subcommand {
        match command {
            ConfigSubcommand::Set { field_path, value } => {
                set::exec(config_path, field_path, value)
                    .await
                    .unwrap_or_else(|e| e.handle_strait_forward())
            }
            ConfigSubcommand::Show { field_path } => show::exec(config_path, field_path)
                .await
                .unwrap_or_else(|e| e.handle_strait_forward()),
            ConfigSubcommand::Unset { field_path } => unset::exec(config_path, field_path)
                .await
                .unwrap_or_else(|e| e.handle_strait_forward()),
        }
    } else {
        unreachable!()
    }
}

mod set {
    use std::path::Path;

    use clap::error::ErrorKind;
    use toml_edit::DocumentMut;

    use crate::{
        app_config::config::AppConfig,
        error::cli::{CliError, CliResult},
    };

    pub(super) async fn exec(
        config_path: String,
        field_path: String,
        value: String,
    ) -> CliResult<()> {
        let map = AppConfig::get_field_value_map();
        // 获取是否存在该字段
        if let Some(kind) = map.get::<str>(field_path.as_ref()) {
            use toml_edit::Item;

            match kind {
                Item::Value(kind) => {
                    let converted_value =
                        parse_value(value, kind).unwrap_or_else(|e| e.handle_strait_forward());

                    // 文件存在就读取文件，文件不存在就创建一个新的
                    let config_content = if Path::new(&config_path).exists() {
                        tokio::fs::read_to_string(&config_path).await?
                    } else {
                        String::new()
                    };

                    let mut doc: DocumentMut = config_content.parse()?;

                    insert_value(&field_path, converted_value, &mut doc);

                    Ok(tokio::fs::write(config_path, doc.to_string()).await?)
                }
                Item::Table(_) => Err(CliError::new(
                    ErrorKind::InvalidValue,
                    format!("You cannot set a whole table in one go! {field_path}"),
                )),
                Item::ArrayOfTables(_) => Err(CliError::new(
                    ErrorKind::InvalidValue,
                    format!("You cannot set a whole table in one go! {field_path}"),
                )),
                Item::None => unreachable!(),
            }
        } else {
            Err(CliError::new(
                ErrorKind::InvalidValue,
                "No such field".to_string(),
            ))
        }
    }

    /// 接下来会有相当多的 unwrap，由于当前配置文件中没有 array，所以可以放心大胆的 unwrap，但是以后必须处理
    ///
    /// 因为 get_mut 和 get 都在两种情况下返回 None：
    ///
    /// 用一个 String 的 index 访问一个数组或者元数据类型
    ///
    /// 或者没有这个字段
    fn insert_value(field_path: &str, converted_value: toml_edit::Item, doc: &mut DocumentMut) {
        let path_parts: Vec<_> = field_path.split('.').collect();
        let mut parrent_node = doc.as_item_mut();
        for (idx, part) in path_parts.iter().enumerate() {
            if idx < path_parts.len() - 1 {
                match parrent_node.get(part) {
                    Some(_) => parrent_node = parrent_node.get_mut(part).unwrap(),
                    None => {
                        parrent_node
                            .as_table_mut()
                            .unwrap()
                            .insert(part, toml_edit::table());
                        parrent_node = parrent_node.get_mut(part).unwrap()
                    }
                }
            } else if idx == path_parts.len() - 1 {
                match parrent_node.get(part) {
                    Some(_) => *parrent_node.get_mut(part).unwrap() = converted_value,
                    None => {
                        parrent_node
                            .as_table_mut()
                            .unwrap()
                            .insert(part, converted_value);
                    }
                }
                break;
            } else {
                unreachable!()
            }
        }
    }

    fn parse_value(value: String, kind: &toml_edit::Value) -> Result<toml_edit::Item, CliError> {
        use toml_edit::Value;
        match kind {
            Value::String(_) => Ok(toml_edit::value(value)),
            Value::Integer(_) => Ok(toml_edit::value(value.parse::<i64>()?)),
            Value::Float(_) => Ok(toml_edit::value(value.parse::<f64>()?)),
            Value::Boolean(_) => Ok(toml_edit::value(value.parse::<bool>()?)),
            Value::Datetime(_) => Ok(toml_edit::value(value.parse::<toml_edit::Datetime>()?)),
            Value::Array(_) => unimplemented!(),
            Value::InlineTable(_) => unimplemented!(),
        }
    }
}

mod show {
    use std::path::Path;

    use clap::{CommandFactory, error::ErrorKind};
    use toml_edit::Document;

    use crate::{app_config::config::AppConfig, cli::Cli, error::cli::CliResult};

    pub(super) async fn exec(config_path: String, field_path: Option<String>) -> CliResult<()> {
        let map = AppConfig::get_valid_paths();
        // 获取是否存在该字段
        if let Some(field_path) = field_path {
            if let Some(kind) = map.get::<str>(field_path.as_ref()) {
                use toml_edit::Item;

                match kind {
                    Item::Value(_) | Item::Table(_) | Item::ArrayOfTables(_) => {
                        // 文件存在就读取文件，文件不存在就创建一个新的
                        let config_content = if Path::new(&config_path).exists() {
                            tokio::fs::read_to_string(&config_path).await?
                        } else {
                            String::new()
                        };

                        let doc: Document<String> = config_content.parse()?;

                        show(&field_path, &doc);
                    }
                    Item::None => unreachable!(),
                }
            } else {
                Cli::command()
                    .error(ErrorKind::InvalidValue, "No such field/table")
                    .exit()
            }
        } else {
            let config_content = if Path::new(&config_path).exists() {
                tokio::fs::read_to_string(&config_path).await?
            } else {
                String::new()
            };

            println!("{config_content}");
        }

        Ok(())
    }

    /// 接下来会有相当多的 unwrap，由于当前配置文件中没有 array，所以可以放心大胆的 unwrap，但是以后必须处理
    ///
    /// 因为 get_mut 和 get 都在两种情况下返回 None：
    ///
    /// 用一个 String 的 index 访问一个数组 (应该使用 usize 访问) 或者元数据类型
    ///
    /// 或者没有这个字段
    fn show(path: &str, doc: &Document<String>) {
        let path_parts: Vec<_> = path.split('.').collect();
        let mut parrent_node = doc.as_item();
        let mut field_value = Some(doc.as_item());
        for (idx, part) in path_parts.iter().enumerate() {
            if idx < path_parts.len() - 1 {
                match parrent_node.get(part) {
                    Some(next_node) => parrent_node = next_node,
                    None => {
                        field_value = None;
                        break;
                    }
                }
            } else if idx == path_parts.len() - 1 {
                field_value = parrent_node.get(part)
            } else {
                unreachable!()
            }
        }

        if let Some(val) = field_value {
            println!("{val}")
        }
    }
}

mod unset {
    use std::path::Path;

    use clap::error::ErrorKind;
    use toml_edit::DocumentMut;

    use crate::{
        app_config::config::AppConfig,
        error::cli::{CliError, CliResult},
    };

    pub(super) async fn exec(config_path: String, field_path: String) -> CliResult<()> {
        let map = AppConfig::get_valid_paths();
        // 获取是否存在该字段
        if let Some(kind) = map.get::<str>(field_path.as_ref()) {
            use toml_edit::Item;

            match kind {
                Item::Value(_) | Item::Table(_) => {
                    // 文件存在就读取文件，文件不存在就创建一个新的
                    let config_content = if Path::new(&config_path).exists() {
                        tokio::fs::read_to_string(&config_path).await?
                    } else {
                        String::new()
                    };

                    let mut doc: DocumentMut = config_content.parse()?;

                    remove_value(&field_path, &mut doc);

                    Ok(tokio::fs::write(config_path, doc.to_string()).await?)
                }
                Item::ArrayOfTables(_) => Err(CliError::new(
                    ErrorKind::InvalidValue,
                    format!("You cannot set a whole table in one go! {field_path}"),
                )),
                Item::None => unreachable!(),
            }
        } else {
            Err(CliError::new(
                ErrorKind::InvalidValue,
                "No such field".to_string(),
            ))
        }
    }

    /// 接下来会有相当多的 unwrap，由于当前配置文件中没有 array，所以可以放心大胆的 unwrap，但是以后必须处理
    ///
    /// 因为 get_mut 和 get 都在两种情况下返回 None：
    ///
    /// 用一个 String 的 index 访问一个数组或者元数据类型
    ///
    /// 或者没有这个字段
    fn remove_value(field_path: &str, doc: &mut DocumentMut) {
        let path_parts: Vec<_> = field_path.split('.').collect();
        let mut parrent_node = doc.as_item_mut();
        for (idx, part) in path_parts.iter().enumerate() {
            if idx < path_parts.len() - 1 {
                match parrent_node.get(part) {
                    Some(_) => parrent_node = parrent_node.get_mut(part).unwrap(),
                    None => {
                        parrent_node
                            .as_table_mut()
                            .unwrap()
                            .insert(part, toml_edit::table());
                        parrent_node = parrent_node.get_mut(part).unwrap()
                    }
                }
            } else if idx == path_parts.len() - 1 {
                match parrent_node.get(part) {
                    Some(_) => {
                        // as_table_link_mut 在 自身是内联表或者是表的时候返回自身的 table_like
                        // 但是在其他情况下返回 None
                        // 又由于没有 array，而且最后一个元素要么是一个表、要么是一个原子结构，所以可以直接 unwrap
                        parrent_node.as_table_like_mut().unwrap().remove(part);
                    }
                    None => return,
                }
                break;
            } else {
                unreachable!()
            }
        }
    }
}
