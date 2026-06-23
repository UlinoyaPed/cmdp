use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub const DEFAULT_INPUT_RECORD_LIMIT: usize = 20;

#[derive(Debug, Clone, Deserialize, Default)]
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
    pub remember_last_selection: bool,
    #[serde(default)]
    pub remember_last_input: bool,
    #[serde(default = "default_input_record_limit")]
    pub input_record_limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Category {
    pub alias: Option<String>,
    #[serde(skip)]
    pub source: Source,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Command {
    pub category: String,
    pub title: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct Param {
    pub name: String,
    pub label: Option<String>,
    pub default: Option<String>,
    pub placeholder: Option<String>,
    pub help: Option<String>,
    #[serde(default)]
    pub secret: bool,
    pub choices: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptionDef {
    pub id: String,
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
