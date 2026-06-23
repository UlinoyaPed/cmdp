use crate::{error::CmdpError, parser, template::*};
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::collections::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn global_dir() -> Result<PathBuf> {
    let base = BaseDirs::new().context("cannot determine home directory")?;
    Ok(base.home_dir().join(".config/cmdp"))
}

pub fn ensure_global_dir() -> Result<()> {
    let dir = global_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

pub fn global_paths() -> Result<Vec<PathBuf>> {
    toml_files_in_dir(&global_dir()?)
}

pub fn settings_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("settings.toml"))
}

pub fn find_local(start: &Path) -> Option<PathBuf> {
    let home = BaseDirs::new().map(|b| b.home_dir().to_path_buf());
    for dir in start.ancestors() {
        let p = dir.join(".cmdp.toml");
        if p.exists() {
            return Some(p);
        }
        if home.as_deref() == Some(dir) {
            break;
        }
    }
    None
}

pub fn load() -> Result<Config> {
    ensure_global_dir()?;
    let mut merged = Config {
        settings: load_settings()?,
        ..Config::default()
    };
    for gp in global_paths()? {
        merge_file(&mut merged, &gp, Source::Global)?;
    }
    if let Some(lp) = find_local(&std::env::current_dir()?) {
        merge_file(&mut merged, &lp, Source::Local)?;
    }
    validate(&merged)?;
    Ok(merged)
}

fn merge_file(merged: &mut Config, path: &Path, source: Source) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(());
    }
    let mut raw: RawConfig =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if raw.version.unwrap_or(1) != 1 {
        return Err(CmdpError::Config(format!("unsupported version in {}", path.display())).into());
    }
    for (_, c) in raw.categories.iter_mut() {
        c.source = source;
    }
    for (_, c) in raw.commands.iter_mut() {
        c.source = source;
    }
    for (id, cat) in raw.categories {
        merged.categories.insert(id, cat);
    }
    for (id, cmd) in raw.commands {
        merged.commands.insert(id, cmd);
    }
    merged
        .sources
        .push(format!("{}:{}", source.label(), path.display()));
    Ok(())
}

fn load_settings() -> Result<Settings> {
    load_settings_from_path(&settings_path()?)
}

fn load_settings_from_path(path: &Path) -> Result<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }

    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Settings::default());
    }

    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn toml_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in
        fs::read_dir(dir).with_context(|| format!("read config directory {}", dir.display()))?
    {
        let path = entry?.path();
        let is_toml = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"));
        if path.is_file() && is_toml && !is_settings_file(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn is_settings_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "settings.toml")
}

fn validate(cfg: &Config) -> Result<()> {
    for id in cfg.categories.keys() {
        validate_id("category", id)?;
    }
    for (id, cmd) in &cfg.commands {
        validate_id("command", id)?;
        if !cfg.categories.contains_key(&cmd.category) {
            return Err(CmdpError::Config(format!(
                "command '{id}' references missing category '{}'",
                cmd.category
            ))
            .into());
        }
        let parsed =
            parser::parse_template(&cmd.template).map_err(|reason| CmdpError::Template {
                command_id: id.clone(),
                reason,
            })?;
        let usage = parser::analyze_template(&parsed);
        let optional_ids: HashSet<_> = usage.optional.iter().map(|opt| opt.id.as_str()).collect();
        let mut param_names = HashSet::new();
        for param in &cmd.params {
            validate_id("parameter", &param.name)?;
            if !param_names.insert(param.name.as_str()) {
                return Err(CmdpError::Config(format!(
                    "command '{id}' defines duplicate parameter '{}'",
                    param.name
                ))
                .into());
            }
        }
        let mut option_ids = HashSet::new();
        for option in &cmd.options {
            validate_id("option", &option.id)?;
            if !option_ids.insert(option.id.as_str()) {
                return Err(CmdpError::Config(format!(
                    "command '{id}' defines duplicate option '{}'",
                    option.id
                ))
                .into());
            }
            if !optional_ids.contains(option.id.as_str()) {
                return Err(CmdpError::Config(format!(
                    "command '{id}' option '{}' does not match any optional template fragment",
                    option.id
                ))
                .into());
            }
        }
    }
    Ok(())
}

fn validate_id(kind: &str, id: &str) -> Result<()> {
    if parser::is_identifier(id) {
        Ok(())
    } else {
        Err(CmdpError::Config(format!("invalid {kind} id '{id}'")).into())
    }
}

#[cfg(test)]
mod tests {
    use super::{load_settings_from_path, merge_file, toml_files_in_dir};
    use crate::template::{Config, Source};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn toml_files_in_dir_filters_and_sorts_toml_files() {
        let dir = temp_config_dir();
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("b.toml"), "").unwrap();
        fs::write(dir.join("a.toml"), "").unwrap();
        fs::write(dir.join("settings.toml"), "").unwrap();
        fs::write(dir.join("README.md"), "").unwrap();
        fs::write(dir.join("nested").join("c.toml"), "").unwrap();

        let paths = toml_files_in_dir(&dir).unwrap();

        assert_eq!(paths, vec![dir.join("a.toml"), dir.join("b.toml")]);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn global_files_are_sorted_and_local_file_overrides_commands() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let global_dir = dir.join("global");
        fs::create_dir_all(&global_dir).unwrap();
        let global_a = global_dir.join("a.toml");
        let global_b = global_dir.join("b.toml");
        let local = dir.join(".cmdp.toml");

        fs::write(
            &global_b,
            r#"
[categories.tools]
alias = "Tools B"

[commands.shared]
category = "tools"
title = "Global B"
template = "echo global-b"
"#,
        )
        .unwrap();
        fs::write(
            &global_a,
            r#"
[categories.tools]
alias = "Tools A"

[commands.first]
category = "tools"
title = "First"
template = "echo first"
"#,
        )
        .unwrap();
        fs::write(
            &local,
            r#"
[categories.tools]
alias = "Local Tools"

[commands.shared]
category = "tools"
title = "Local Shared"
template = "echo local"
"#,
        )
        .unwrap();

        let mut merged = Config::default();
        for path in toml_files_in_dir(&global_dir).unwrap() {
            merge_file(&mut merged, &path, Source::Global).unwrap();
        }
        merge_file(&mut merged, &local, Source::Local).unwrap();

        assert_eq!(
            merged.sources,
            vec![
                format!("global:{}", global_a.display()),
                format!("global:{}", global_b.display()),
                format!("local:{}", local.display()),
            ]
        );
        assert_eq!(
            merged
                .categories
                .get("tools")
                .and_then(|category| category.alias.as_deref()),
            Some("Local Tools")
        );
        let shared = merged.commands.get("shared").unwrap();
        assert_eq!(shared.title.as_deref(), Some("Local Shared"));
        assert_eq!(shared.template, "echo local");
        assert_eq!(shared.source, Source::Local);
        assert!(merged.commands.contains_key("first"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn settings_are_loaded_from_dedicated_file() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let settings = dir.join("settings.toml");
        fs::write(
            &settings,
            r#"
remember_last_selection = true
remember_last_input = true
input_record_limit = 7
"#,
        )
        .unwrap();

        let settings = load_settings_from_path(&settings).unwrap();

        assert!(settings.remember_last_selection);
        assert!(settings.remember_last_input);
        assert_eq!(settings.input_record_limit, 7);

        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_config_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cmdp-config-test-{}-{nonce}", std::process::id()))
    }
}
