use anyhow::{Context, Result};
use dirs::{config_dir, data_dir};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub save_dir: PathBuf,
}

static PATHS: OnceCell<AppPaths> = OnceCell::new();

pub fn init_logging() {
    static LOGGER: OnceCell<()> = OnceCell::new();
    LOGGER.get_or_init(|| {
        let filter = std::env::var("RMXP_LOG").unwrap_or_else(|_| "info".into());
        let env_filter = EnvFilter::new(filter);
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .init();
    });
}

pub fn app_paths() -> Result<&'static AppPaths> {
    PATHS.get_or_try_init(|| {
        let config = config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rmxp-native-player");
        let save = data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rmxp-native-player");
        fs::create_dir_all(&config).context("creating config dir")?;
        fs::create_dir_all(&save).context("creating save dir")?;
        info!(
            target: "platform::paths",
            config = ?config,
            save = ?save,
            "paths initialised"
        );
        Ok(AppPaths {
            config_dir: config,
            save_dir: save,
        })
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineConfig {
    pub window_width: u32,
    pub window_height: u32,
    pub title: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            window_width: 1024,
            window_height: 768,
            title: "RMXP Native Player".to_string(),
        }
    }
}

pub fn load_or_default() -> EngineConfig {
    let paths = match app_paths() {
        Ok(p) => p,
        Err(_) => return EngineConfig::default(),
    };
    let cfg_path = paths.config_dir.join("config.toml");
    if let Ok(contents) = fs::read_to_string(&cfg_path) {
        if let Ok(cfg) = toml::from_str(&contents) {
            return cfg;
        }
    }
    let default = EngineConfig::default();
    if let Ok(serialized) = toml::to_string_pretty(&default) {
        let _ = fs::write(cfg_path, serialized);
    }
    default
}
