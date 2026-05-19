use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::game::{CpuMode, ThemePreference};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub gcn_library: Option<PathBuf>,
    pub wii_library: Option<PathBuf>,
    pub cpu_mode: CpuMode,
    pub theme: ThemePreference,
}

pub fn config_path() -> PathBuf {
    match std::env::current_exe() {
        Ok(exe) => exe
            .parent()
            .map(|p| p.join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml")),
        Err(_) => PathBuf::from("config.toml"),
    }
}

pub fn load(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(s) => match toml::from_str(&s) {
            Ok(cfg) => cfg,
            Err(err) => {
                tracing::warn!(?err, path = %path.display(), "failed to parse config; using defaults");
                Config::default()
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(err) => {
            tracing::warn!(?err, path = %path.display(), "failed to read config; using defaults");
            Config::default()
        }
    }
}

pub fn save(path: &Path, cfg: &Config) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let body = toml::to_string_pretty(cfg)?;
    std::fs::write(path, body)?;
    Ok(())
}
