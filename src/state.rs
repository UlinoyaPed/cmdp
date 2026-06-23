use crate::template::AppState;
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn load() -> Result<Option<AppState>> {
    read_from_path(&state_path()?)
}

pub fn save(state: &AppState) -> Result<()> {
    write_to_path(&state_path()?, state)
}

pub fn state_path() -> Result<PathBuf> {
    let base = BaseDirs::new().context("cannot determine home directory")?;
    let state_dir = base
        .state_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| base.home_dir().join(".local/state"));
    Ok(state_dir.join("cmdp").join("state.toml"))
}

fn read_from_path(path: &Path) -> Result<Option<AppState>> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(None),
        Ok(text) => toml::from_str(&text)
            .map(Some)
            .with_context(|| format!("parse {}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn write_to_path(path: &Path, state: &AppState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let text = toml::to_string(state).context("serialize app state")?;
    fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{read_from_path, write_to_path};
    use crate::template::{AppState, InputRecord};
    use std::collections::BTreeMap;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn app_state_round_trips_as_toml() {
        let dir = temp_state_dir();
        let path = dir.join("state.toml");
        let mut values = BTreeMap::new();
        values.insert("path".to_string(), "./src".to_string());
        let state = AppState {
            category_id: Some("dev".to_string()),
            command_id: Some("cargo_test".to_string()),
            input_records: vec![InputRecord {
                command_id: "cargo_test".to_string(),
                values,
                enabled: vec!["locked".to_string()],
            }],
        };

        write_to_path(&path, &state).unwrap();

        assert_eq!(read_from_path(&path).unwrap(), Some(state));
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_state_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cmdp-state-test-{}-{nonce}", std::process::id()))
    }
}
