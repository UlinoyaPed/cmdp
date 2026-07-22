use crate::{error::CmdpError, parser, template::*};
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::collections::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_SETTINGS_TOML: &str = r#"# cmdp settings
language = "zh-CN"
remember_last_selection = false
remember_last_input = false
input_record_limit = 20
"#;

const DEFAULT_COMMANDS_TOML: &str = r#"version = 1

[categories.general]
alias = "常用命令"

[commands.list_files]
category = "general"
title = "列出文件"
description = "列出指定目录下的文件"
danger = false
template = '''
ls [[all:-a]] [[long:-l]] <<"{{path}}">>
'''

params = [
  { name = "path", label = "路径", default = ".", placeholder = "." },
]

options = [
  { id = "all", label = "显示隐藏文件 -a", default_enabled = false },
  { id = "long", label = "详细列表 -l", default_enabled = true },
]
"#;

const EDITOR_CONFIG_FILE: &str = "zz_cmdp_editor.toml";
const MAX_INPUT_RECORD_LIMIT: usize = 10_000;

#[derive(Debug, Clone)]
pub struct CommandEdit {
    pub original_command_id: Option<String>,
    pub command_id: String,
    pub category_id: String,
    pub category_alias: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub danger: bool,
    pub template: String,
    pub params: Vec<Param>,
    pub options: Vec<OptionDef>,
}

pub fn global_dir() -> Result<PathBuf> {
    let base = BaseDirs::new().context("cannot determine home directory")?;
    Ok(select_config_dir(
        base.config_dir().join("cmdp"),
        base.home_dir().join(".config/cmdp"),
    ))
}

fn select_config_dir(xdg: PathBuf, legacy: PathBuf) -> PathBuf {
    if xdg.exists() || !legacy.exists() {
        xdg
    } else {
        legacy
    }
}

fn ensure_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn ensure_global_files() -> Result<()> {
    ensure_global_files_in_dir(&global_dir()?)
}

fn ensure_global_files_in_dir(dir: &Path) -> Result<()> {
    ensure_dir(dir)?;

    let settings = dir.join("settings.toml");
    if !settings.exists() {
        crate::atomic::write(&settings, DEFAULT_SETTINGS_TOML.as_bytes(), false)
            .with_context(|| format!("write {}", settings.display()))?;
    }

    if toml_files_in_dir(dir)?.is_empty() {
        let commands = dir.join("commands.toml");
        crate::atomic::write(&commands, DEFAULT_COMMANDS_TOML.as_bytes(), false)
            .with_context(|| format!("write {}", commands.display()))?;
    }

    Ok(())
}

pub fn global_paths() -> Result<Vec<PathBuf>> {
    toml_files_in_dir(&global_dir()?)
}

pub fn settings_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("settings.toml"))
}

pub fn editor_config_path() -> Result<PathBuf> {
    Ok(global_dir()?.join(EDITOR_CONFIG_FILE))
}

pub fn find_global_command_path(command_id: &str) -> Result<Option<PathBuf>> {
    let mut found = None;
    for path in global_paths()? {
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        if text.trim().is_empty() {
            continue;
        }
        let raw: RawConfig =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        if raw.commands.contains_key(command_id) {
            found = Some(path);
        }
    }
    Ok(found)
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
    ensure_global_files()?;
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

pub fn save_settings(settings: &Settings) -> Result<()> {
    let dir = global_dir()?;
    ensure_dir(&dir)?;
    save_settings_to_path(&dir.join("settings.toml"), settings)
}

pub fn save_command_edit(edit: &CommandEdit) -> Result<()> {
    let path = editor_config_path()?;
    if let Some(dir) = path.parent() {
        ensure_dir(dir)?;
    }
    save_command_edit_to_path(&path, edit)
}

pub fn save_command_edit_to_local_path(path: &Path, edit: &CommandEdit) -> Result<()> {
    if let Some(dir) = path.parent() {
        ensure_dir(dir)?;
    }
    save_command_edit_to_path(path, edit)
}

fn load_settings_from_path(path: &Path) -> Result<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }

    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Settings::default());
    }

    let settings: Settings =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if settings.input_record_limit > MAX_INPUT_RECORD_LIMIT {
        return Err(CmdpError::Config(format!(
            "input_record_limit in {} exceeds maximum {MAX_INPUT_RECORD_LIMIT}",
            path.display()
        ))
        .into());
    }
    Ok(settings)
}

fn save_settings_to_path(path: &Path, settings: &Settings) -> Result<()> {
    let text = settings_to_toml(settings);
    crate::atomic::write(path, text.as_bytes(), false)
        .with_context(|| format!("write {}", path.display()))
}

fn save_command_edit_to_path(path: &Path, edit: &CommandEdit) -> Result<()> {
    use toml_edit::{DocumentMut, Item};
    let original = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?
    } else {
        String::new()
    };
    let mut document = if original.trim().is_empty() {
        "version = 1\n".parse::<DocumentMut>()?
    } else {
        original
            .parse::<DocumentMut>()
            .with_context(|| format!("parse {}", path.display()))?
    };
    let existing: RawConfig = toml::from_str(&document.to_string())
        .with_context(|| format!("parse {}", path.display()))?;
    if existing.version.unwrap_or(1) != 1 {
        return Err(CmdpError::Config(format!("unsupported version in {}", path.display())).into());
    }
    if edit.original_command_id.as_deref() != Some(edit.command_id.as_str())
        && existing.commands.contains_key(&edit.command_id)
    {
        return Err(CmdpError::Config(format!(
            "command id '{}' already exists in {}",
            edit.command_id,
            path.display()
        ))
        .into());
    }

    let mut fragment = RawConfig::default();
    fragment.categories.insert(
        edit.category_id.clone(),
        Category {
            alias: edit.category_alias.clone(),
            source: Source::Global,
        },
    );
    fragment.commands.insert(
        edit.command_id.clone(),
        Command {
            category: edit.category_id.clone(),
            title: edit.title.clone(),
            description: edit.description.clone(),
            danger: edit.danger,
            template: edit.template.clone(),
            params: edit.params.clone(),
            options: edit.options.clone(),
            source: Source::Global,
        },
    );
    let fragment = toml_edit::ser::to_document(&fragment)?;
    if !document.contains_key("categories") {
        document["categories"] = Item::Table(Default::default());
    }
    if !document.contains_key("commands") {
        document["commands"] = Item::Table(Default::default());
    }
    if document["categories"].get(&edit.category_id).is_none() {
        document["categories"][&edit.category_id] = Item::Table(Default::default());
    }
    if edit.category_alias.is_some() {
        let target = &mut document["categories"][&edit.category_id]["alias"];
        replace_item_preserving_decor(target, &fragment["categories"][&edit.category_id]["alias"]);
    }
    if let Some(old) = edit
        .original_command_id
        .as_deref()
        .filter(|old| *old != edit.command_id)
    {
        let previous = document["commands"]
            .as_table_like_mut()
            .context("commands must be a table")?
            .remove(old);
        if let Some(previous) = previous {
            document["commands"][&edit.command_id] = previous;
        }
    }
    if document["commands"].get(&edit.command_id).is_none() {
        document["commands"][&edit.command_id] = Item::Table(Default::default());
    }
    let target = &mut document["commands"][&edit.command_id];
    let source = &fragment["commands"][&edit.command_id];
    for field in ["category", "danger", "template", "params", "options"] {
        replace_item_preserving_decor(&mut target[field], &source[field]);
    }
    for field in ["title", "description"] {
        if source.get(field).is_some() {
            replace_item_preserving_decor(&mut target[field], &source[field]);
        } else if let Some(table) = target.as_table_like_mut() {
            table.remove(field);
        }
    }

    let text = document.to_string();
    let raw: RawConfig =
        toml::from_str(&text).with_context(|| format!("validate {}", path.display()))?;
    let cfg = Config {
        categories: raw.categories,
        commands: raw.commands,
        ..Config::default()
    };
    validate(&cfg)?;
    crate::atomic::write(path, text.as_bytes(), false)
        .with_context(|| format!("write {}", path.display()))
}

fn replace_item_preserving_decor(target: &mut toml_edit::Item, source: &toml_edit::Item) {
    if let (Some(existing), Some(replacement)) = (target.as_value_mut(), source.as_value()) {
        let decor = existing.decor().clone();
        *existing = replacement.clone();
        *existing.decor_mut() = decor;
    } else {
        *target = source.clone();
    }
}

fn settings_to_toml(settings: &Settings) -> String {
    format!(
        r#"language = "{}"
remember_last_selection = {}
remember_last_input = {}
input_record_limit = {}
"#,
        settings.language.code(),
        settings.remember_last_selection,
        settings.remember_last_input,
        settings.input_record_limit
    )
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
        if cmd.template.trim().is_empty() {
            return Err(CmdpError::Config(format!("command '{id}' has an empty template")).into());
        }
        let parsed =
            parser::parse_template(&cmd.template).map_err(|reason| CmdpError::Template {
                command_id: id.clone(),
                reason,
            })?;
        let usage = parser::analyze_template(&parsed);
        let optional_ids: HashSet<_> = usage.optional.iter().map(|opt| opt.id.as_str()).collect();
        let used_params: HashSet<_> = usage
            .required_params
            .iter()
            .map(String::as_str)
            .chain(
                usage
                    .optional
                    .iter()
                    .flat_map(|opt| opt.params.iter().map(String::as_str)),
            )
            .collect();
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
            if let Some(choices) = &param.choices {
                if choices.is_empty() {
                    return Err(CmdpError::Config(format!(
                        "command '{id}' parameter '{}' has empty choices",
                        param.name
                    ))
                    .into());
                }
                if param.secret {
                    return Err(CmdpError::Config(format!(
                        "command '{id}' secret parameter '{}' cannot define choices",
                        param.name
                    ))
                    .into());
                }
                if let Some(default) = &param.default
                    && !choices.contains(default)
                {
                    return Err(CmdpError::Config(format!(
                        "command '{id}' parameter '{}' default is not in choices",
                        param.name
                    ))
                    .into());
                }
            }
        }
        for used in &used_params {
            if !param_names.contains(used) {
                return Err(CmdpError::Config(format!(
                    "command '{id}' template uses undefined parameter '{used}'"
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
        for optional in optional_ids {
            if !option_ids.contains(optional) {
                return Err(CmdpError::Config(format!(
                    "command '{id}' optional fragment '{optional}' has no option definition"
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
    use super::{
        CommandEdit, ensure_global_files_in_dir, load_settings_from_path, merge_file,
        save_command_edit_to_path, save_settings_to_path, select_config_dir, toml_files_in_dir,
    };
    use crate::{
        i18n::Language,
        template::{Config, OptionDef, Param, Settings, Source},
    };
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
language = "en"
"#,
        )
        .unwrap();

        let settings = load_settings_from_path(&settings).unwrap();

        assert!(settings.remember_last_selection);
        assert!(settings.remember_last_input);
        assert_eq!(settings.input_record_limit, 7);
        assert_eq!(settings.language, Language::En);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn settings_are_saved_to_dedicated_file() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let settings_path = dir.join("settings.toml");

        save_settings_to_path(
            &settings_path,
            &Settings {
                language: Language::En,
                remember_last_selection: true,
                remember_last_input: true,
                input_record_limit: 3,
            },
        )
        .unwrap();

        let loaded = load_settings_from_path(&settings_path).unwrap();

        assert_eq!(loaded.language, Language::En);
        assert!(loaded.remember_last_selection);
        assert!(loaded.remember_last_input);
        assert_eq!(loaded.input_record_limit, 3);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn startup_config_files_are_generated_when_missing() {
        let dir = temp_config_dir();

        ensure_global_files_in_dir(&dir).unwrap();

        let settings = dir.join("settings.toml");
        let commands = dir.join("commands.toml");
        assert!(settings.exists());
        assert!(commands.exists());

        let settings = load_settings_from_path(&settings).unwrap();
        assert_eq!(settings.language, Language::ZhCn);

        let mut merged = Config::default();
        merge_file(&mut merged, &commands, Source::Global).unwrap();
        assert!(merged.categories.contains_key("general"));
        assert!(merged.commands.contains_key("list_files"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn startup_config_generation_keeps_existing_command_files() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let custom = dir.join("custom.toml");
        fs::write(
            &custom,
            r#"
[categories.tools]
alias = "Tools"

[commands.echo]
category = "tools"
title = "Echo"
template = "echo ok"
"#,
        )
        .unwrap();

        ensure_global_files_in_dir(&dir).unwrap();

        assert!(dir.join("settings.toml").exists());
        assert!(!dir.join("commands.toml").exists());
        assert!(custom.exists());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn command_edits_are_saved_to_editor_config_file() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("zz_cmdp_editor.toml");

        save_command_edit_to_path(
            &path,
            &CommandEdit {
                original_command_id: None,
                command_id: "say_hello".to_string(),
                category_id: "general".to_string(),
                category_alias: Some("General".to_string()),
                title: Some("Say Hello".to_string()),
                description: Some("Greets a name".to_string()),
                danger: false,
                template: "echo [[shout:--shout]] <<{{name}}>>".to_string(),
                params: vec![Param {
                    name: "name".to_string(),
                    label: Some("Name".to_string()),
                    default: None,
                    placeholder: None,
                    help: None,
                    secret: false,
                    choices: None,
                }],
                options: vec![OptionDef {
                    id: "shout".to_string(),
                    label: Some("Shout".to_string()),
                    default_enabled: true,
                }],
            },
        )
        .unwrap();

        let mut merged = Config::default();
        merge_file(&mut merged, &path, Source::Global).unwrap();

        assert!(merged.categories.contains_key("general"));
        let command = merged.commands.get("say_hello").unwrap();
        assert_eq!(command.title.as_deref(), Some("Say Hello"));
        assert_eq!(command.description.as_deref(), Some("Greets a name"));
        assert_eq!(command.params[0].name, "name");
        assert_eq!(command.options[0].id, "shout");
        assert!(command.options[0].default_enabled);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn command_edit_preserves_comments_and_unknown_fields() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("commands.toml");
        fs::write(
            &path,
            r#"# keep this comment
version = 1
future_setting = "keep"

[categories.general]
alias = "General" # category comment

[commands.old]
category = "general"
template = "echo old"
future_command_field = 42
"#,
        )
        .unwrap();
        let mut edit = sample_edit("old");
        edit.original_command_id = Some("old".into());
        edit.title = Some("Changed".into());
        save_command_edit_to_path(&path, &edit).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("# keep this comment"));
        assert!(saved.contains("future_setting = \"keep\""));
        assert!(saved.contains("future_command_field = 42"));
        assert!(saved.contains("# category comment"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn renaming_command_removes_old_id_and_rejects_collision_without_writing() {
        let dir = temp_config_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("commands.toml");
        fs::write(
            &path,
            r#"version = 1
[categories.general]
alias = "General"
[commands.old]
category = "general"
template = "echo old"
[commands.taken]
category = "general"
template = "echo taken"
"#,
        )
        .unwrap();
        let before = fs::read_to_string(&path).unwrap();
        let mut collision = sample_edit("taken");
        collision.original_command_id = Some("old".into());
        assert!(save_command_edit_to_path(&path, &collision).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before);

        let mut rename = sample_edit("renamed");
        rename.original_command_id = Some("old".into());
        save_command_edit_to_path(&path, &rename).unwrap();
        let parsed: crate::template::RawConfig =
            toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(!parsed.commands.contains_key("old"));
        assert!(parsed.commands.contains_key("renamed"));
        assert!(parsed.commands.contains_key("taken"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn xdg_directory_wins_and_legacy_is_used_only_as_fallback() {
        let dir = temp_config_dir();
        let xdg = dir.join("xdg/cmdp");
        let legacy = dir.join("home/.config/cmdp");
        assert_eq!(select_config_dir(xdg.clone(), legacy.clone()), xdg);
        fs::create_dir_all(&legacy).unwrap();
        assert_eq!(select_config_dir(xdg.clone(), legacy.clone()), legacy);
        fs::create_dir_all(&xdg).unwrap();
        assert_eq!(select_config_dir(xdg.clone(), legacy), xdg);
        fs::remove_dir_all(dir).unwrap();
    }

    fn sample_edit(id: &str) -> CommandEdit {
        CommandEdit {
            original_command_id: None,
            command_id: id.into(),
            category_id: "general".into(),
            category_alias: Some("General".into()),
            title: Some("Title".into()),
            description: None,
            danger: false,
            template: "echo ok".into(),
            params: vec![],
            options: vec![],
        }
    }

    fn temp_config_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cmdp-config-test-{}-{nonce}", std::process::id()))
    }
}
