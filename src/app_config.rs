use std::{process::exit, sync::LazyLock};

use clap::Parser;
use serde::Deserialize;

pub static CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let default_conf = AppConfig::default();
    let cli_conf = AppConfig::parse();
    let file_conf: AppConfig = config::Config::builder()
    .add_source(
        config::File::with_name("crab-vault.toml")
            .required(false)
            .format(config::FileFormat::Toml),
    )
    .build()
    .unwrap_or_else(|e| {
        println!("无法读取配置文件！{e}");
        exit(1);
    })
    .try_deserialize()
    .unwrap_or_else(|e| {
        println!("无法读取配置文件！{e}");
        exit(1);
    });

    let curr_conf = default_conf;
    let curr_conf = file_conf.overwrite(curr_conf);
    let curr_conf = cli_conf.overwrite(curr_conf);

    curr_conf
});

mod default {
    pub(super) const PORT: u16 = 32767;
    pub(super) const DATA_MNT_POINT: &str = "./data";
    pub(super) const META_MNT_POINT: &str = "./meta";
}

#[derive(Parser, Deserialize)]
#[command(version, author, about, long_about = None)]
#[command(name = "Crab Vault 🦀📦")]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// 指定服务器守候的端口
    #[arg(long = "port", short = 'p')]
    port: Option<u16>,

    /// 指定文件的存储位置
    #[arg(long = "data-mount-point", short = 'D')]
    data_mnt_point: Option<String>,

    /// 指定元数据 metadata 的存储位置
    #[arg(long = "meta-mount-point", short = 'M')]
    meta_mnt_point: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: Some(default::PORT),
            data_mnt_point: Some(default::DATA_MNT_POINT.to_string()),
            meta_mnt_point: Some(default::META_MNT_POINT.to_string()),
        }
    }
}

impl AppConfig {
    fn overwrite(self, rhs: Self) -> Self {
        Self {
            port: self.port.or(rhs.port),
            data_mnt_point: self.data_mnt_point.or(rhs.data_mnt_point),
            meta_mnt_point: self.meta_mnt_point.or(rhs.meta_mnt_point),
        }
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or(32767)
    }

    pub fn data_mnt_point(&self) -> &str {
        match &self.data_mnt_point {
            Some(val) => &val,
            None => "./data"
        }
    }

    #[allow(dead_code)]
    pub fn meta_mnt_point(&self) -> &str {
        match &self.meta_mnt_point {
            Some(val) => &val,
            None => "./meta"
        }
    }
}
