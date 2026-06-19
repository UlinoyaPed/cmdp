use crate::{error::CmdpError, parser, template::*};
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::collections::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn global_path() -> Result<PathBuf> {
    let base = BaseDirs::new().context("cannot determine home directory")?;
    Ok(base.home_dir().join(".config/cmdp/commands.toml"))
}

pub fn ensure_example_global() -> Result<()> {
    let path = global_path()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            if parent.exists() {
                return Ok(());
            }
            fs::create_dir_all(parent)?;
        }
        let sample = include_str!("../examples/commands.toml");
        fs::write(path, sample)?;
    }
    Ok(())
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
    ensure_example_global()?;
    let mut merged = Config::default();
    let gp = global_path()?;
    if gp.exists() {
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
