use crate::i18n::Language;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub const DEFAULT_INPUT_RECORD_LIMIT: usize = 20;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RawConfig {
    pub version: Option<u32>,
    #[serde(default)]
    pub categories: IndexMap<String, Category>,
    #[serde(default)]
    pub commands: IndexMap<String, Command>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub remember_last_selection: bool,
    #[serde(default)]
    pub remember_last_input: bool,
    #[serde(default = "default_input_record_limit")]
    pub input_record_limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            remember_last_selection: false,
            remember_last_input: false,
            input_record_limit: DEFAULT_INPUT_RECORD_LIMIT,
        }
    }
}

fn default_input_record_limit() -> usize {
    DEFAULT_INPUT_RECORD_LIMIT
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_records: Vec<InputRecord>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct InputRecord {
    pub command_id: String,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub values: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Category {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip)]
    pub source: Source,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Command {
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub danger: bool,
    pub template: String,
    #[serde(default)]
    pub params: Vec<Param>,
    #[serde(default)]
    pub options: Vec<OptionDef>,
    #[serde(skip)]
    pub source: Source,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Param {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default)]
    pub secret: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionDef {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub default_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    Global,
    Local,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Global => "global",
            Source::Local => "local",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub settings: Settings,
    pub categories: IndexMap<String, Category>,
    pub commands: IndexMap<String, Command>,
    pub sources: Vec<String>,
}
