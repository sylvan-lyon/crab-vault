use clap::Subcommand;

#[derive(Subcommand, Clone, Debug)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
pub enum Command {
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

pub(super) async fn exec(config_path: String, command: Command) {
    match command {
        Command::Set { field_path, value } => set::exec(config_path, field_path, value)
            .await
            .unwrap_or_else(|e| e.handle_strait_forward()),
        Command::Show { field_path } => show::exec(config_path, field_path)
            .await
            .unwrap_or_else(|e| e.handle_strait_forward()),
        Command::Unset { field_path } => unset::exec(config_path, field_path)
            .await
            .unwrap_or_else(|e| e.handle_strait_forward()),
    }
}

mod set {
    use std::path::Path;

    use clap::error::ErrorKind;
    use toml_edit::DocumentMut;

    use crate::{
        app_config::AppConfig,
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
                    None,
                )),
                Item::ArrayOfTables(_) => Err(CliError::new(
                    ErrorKind::InvalidValue,
                    format!("You cannot set a whole table in one go! {field_path}"),
                    None,
                )),
                Item::None => unreachable!(),
            }
        } else {
            Err(CliError::new(
                ErrorKind::InvalidValue,
                "No such field".to_string(),
                None,
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
        let mut parent_node = doc.as_item_mut();
        for (idx, part) in path_parts.iter().enumerate() {
            if idx < path_parts.len() - 1 {
                match parent_node.get(part) {
                    Some(_) => parent_node = parent_node.get_mut(part).unwrap(),
                    None => {
                        parent_node
                            .as_table_mut()
                            .unwrap()
                            .insert(part, toml_edit::table());
                        parent_node = parent_node.get_mut(part).unwrap()
                    }
                }
            } else if idx == path_parts.len() - 1 {
                match parent_node.get(part) {
                    Some(_) => *parent_node.get_mut(part).unwrap() = converted_value,
                    None => {
                        parent_node
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

    use crate::{app_config::AppConfig, cli::Cli, error::cli::CliResult};

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
        let mut parent_node = doc.as_item();
        let mut field_value = Some(doc.as_item());
        for (idx, part) in path_parts.iter().enumerate() {
            if idx < path_parts.len() - 1 {
                match parent_node.get(part) {
                    Some(next_node) => parent_node = next_node,
                    None => {
                        field_value = None;
                        break;
                    }
                }
            } else if idx == path_parts.len() - 1 {
                field_value = parent_node.get(part)
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
        app_config::AppConfig,
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
                    None,
                )),
                Item::None => unreachable!(),
            }
        } else {
            Err(CliError::new(
                ErrorKind::InvalidValue,
                "No such field".to_string(),
                None,
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
        let mut parent_node = doc.as_item_mut();
        for (idx, part) in path_parts.iter().enumerate() {
            if idx < path_parts.len() - 1 {
                match parent_node.get(part) {
                    Some(_) => parent_node = parent_node.get_mut(part).unwrap(),
                    None => {
                        parent_node
                            .as_table_mut()
                            .unwrap()
                            .insert(part, toml_edit::table());
                        parent_node = parent_node.get_mut(part).unwrap()
                    }
                }
            } else if idx == path_parts.len() - 1 {
                match parent_node.get(part) {
                    Some(_) => {
                        // as_table_link_mut 在 自身是内联表或者是表的时候返回自身的 table_like
                        // 但是在其他情况下返回 None
                        // 又由于没有 array，而且最后一个元素要么是一个表、要么是一个原子结构，所以可以直接 unwrap
                        parent_node.as_table_like_mut().unwrap().remove(part);
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
