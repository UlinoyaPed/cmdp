use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawConfig {
    pub version: Option<u32>,
    #[serde(default)]
    pub categories: IndexMap<String, Category>,
    #[serde(default)]
    pub commands: IndexMap<String, Command>,
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
    pub categories: IndexMap<String, Category>,
    pub commands: IndexMap<String, Command>,
    pub sources: Vec<String>,
}
