//! This file is used in order to handle api key in config 
//! 
//! Should save the api key to `~/.config/lastui/config.toml`

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub username: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    String::from("catppuccin-mocha")
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lastui")
        .join("config.toml")
}

pub fn load() -> Option<Config> {
    let path = config_path();
    let contents = fs::read_to_string(path).ok()?;
    toml::from_str::<Config>(&contents).ok()
}

pub fn save(api_key: &str, username: &str, theme: &str) -> anyhow::Result<()> {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap())?;
    let config = Config { 
        api_key: api_key.to_string(), 
        username: username.to_string(),
        theme: theme.to_string(),
    };
    fs::write(path, toml::to_string(&config)?)?;
    Ok(())
}
