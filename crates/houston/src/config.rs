use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub pinned: Vec<String>,
}

fn config_path() -> Option<PathBuf> {
    // XDG_CONFIG_HOME or ~/.config
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("houston").join("config.toml"))
}

pub fn load_pins() -> HashSet<String> {
    let path = match config_path() {
        Some(p) => p,
        None => return HashSet::new(),
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashSet::new(),
    };
    toml::from_str::<Config>(&contents)
        .map(|c| c.pinned.into_iter().collect())
        .unwrap_or_default()
}

pub fn save_pins(pins: &HashSet<String>) {
    let path = match config_path() {
        Some(p) => p,
        None => return,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut pinned: Vec<String> = pins.iter().cloned().collect();
    pinned.sort();
    let config = Config { pinned };
    if let Ok(contents) = toml::to_string_pretty(&config) {
        let _ = std::fs::write(&path, contents);
    }
}
