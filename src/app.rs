use crate::{
    config,
    parser::{self, ParsedTemplate},
    preview, renderer,
    template::*,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Categories,
    Commands,
    Form,
    Preview,
}

pub struct App {
    pub config: Config,
    pub category_idx: usize,
    pub command_idx: usize,
    pub form_idx: usize,
    pub focus: Focus,
    pub editing: bool,
    pub edit_buffer: String,
    pub values: HashMap<String, String>,
    pub enabled: HashSet<String>,
    pub should_quit: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut app = Self {
            config,
            category_idx: 0,
            command_idx: 0,
            form_idx: 0,
            focus: Focus::Categories,
            editing: false,
            edit_buffer: String::new(),
            values: HashMap::new(),
            enabled: HashSet::new(),
            should_quit: false,
            output: None,
            error: None,
        };
        app.reset_form();
        app
    }
    pub fn reload(&mut self) {
        match config::load() {
            Ok(c) => {
                self.config = c;
                self.category_idx = 0;
                self.command_idx = 0;
                self.reset_form();
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    pub fn category_ids(&self) -> Vec<&String> {
        self.config.categories.keys().collect()
    }
    pub fn current_category_id(&self) -> Option<&String> {
        self.category_ids().get(self.category_idx).copied()
    }
    pub fn commands_in_category(&self) -> Vec<(&String, &Command)> {
        let cat = self.current_category_id().cloned();
        self.config
            .commands
            .iter()
            .filter(|(_, c)| Some(&c.category) == cat.as_ref())
            .collect()
    }
    pub fn current_command(&self) -> Option<(&String, &Command)> {
        self.commands_in_category()
            .get(self.command_idx)
            .map(|(id, c)| (*id, *c))
    }
    pub fn parsed(&self) -> Option<ParsedTemplate> {
        self.current_command()
            .and_then(|(_, c)| parser::parse_template(&c.template).ok())
    }
    pub fn reset_form(&mut self) {
        self.values.clear();
        self.enabled.clear();
        self.form_idx = 0;
        if let Some((_, cmd)) = self.current_command() {
            let params = cmd.params.clone();
            let opts = cmd.options.clone();
            for p in &params {
                self.values
                    .insert(p.name.clone(), p.default.clone().unwrap_or_default());
            }
            for o in &opts {
                if o.default_enabled {
                    self.enabled.insert(o.id.clone());
                }
            }
        }
    }
    pub fn form_len(&self) -> usize {
        self.current_command()
            .map(|(_, c)| c.params.len() + c.options.len())
            .unwrap_or(0)
    }
    pub fn next_focus(&mut self, rev: bool) {
        self.focus = match (self.focus, rev) {
            (Focus::Categories, false) => Focus::Commands,
            (Focus::Commands, false) => Focus::Form,
            (Focus::Form, false) => Focus::Preview,
            (Focus::Preview, false) => Focus::Categories,
            (Focus::Categories, true) => Focus::Preview,
            (Focus::Commands, true) => Focus::Categories,
            (Focus::Form, true) => Focus::Commands,
            (Focus::Preview, true) => Focus::Form,
        };
    }
    pub fn move_sel(&mut self, down: bool) {
        let delta = if down { 1isize } else { -1 };
        match self.focus {
            Focus::Categories => {
                let n = self.category_ids().len();
                self.category_idx = step(self.category_idx, n, delta);
                self.command_idx = 0;
                self.reset_form();
            }
            Focus::Commands => {
                let n = self.commands_in_category().len();
                self.command_idx = step(self.command_idx, n, delta);
                self.reset_form();
            }
            Focus::Form => {
                self.form_idx = step(self.form_idx, self.form_len(), delta);
            }
            Focus::Preview => {}
        }
    }
    pub fn activate(&mut self) {
        if self.focus == Focus::Form {
            let param = self
                .current_command()
                .and_then(|(_, cmd)| cmd.params.get(self.form_idx).cloned());
            if let Some(p) = param {
                if let Some(choices) = &p.choices {
                    if !choices.is_empty() {
                        cycle_choice(&mut self.values, &p.name, choices);
                        return;
                    }
                }
                self.editing = true;
                self.edit_buffer = self.values.get(&p.name).cloned().unwrap_or_default();
            }
        }
    }

    pub fn toggle(&mut self) {
        let opt_id = self.current_command().and_then(|(_, cmd)| {
            if self.focus == Focus::Form && self.form_idx >= cmd.params.len() {
                cmd.options
                    .get(self.form_idx - cmd.params.len())
                    .map(|o| o.id.clone())
            } else {
                None
            }
        });
        if let Some(id) = opt_id {
            if !self.enabled.remove(&id) {
                self.enabled.insert(id);
            }
        }
    }

    pub fn commit_edit(&mut self) {
        if let Some((_, cmd)) = self.current_command() {
            if let Some(p) = cmd.params.get(self.form_idx) {
                self.values.insert(p.name.clone(), self.edit_buffer.clone());
            }
        }
        self.editing = false;
    }
    pub fn render(&self, mask: bool) -> Option<renderer::Rendered> {
        let tpl = self.parsed()?;
        let secrets = self
            .current_command()
            .map(|(_, c)| {
                c.params
                    .iter()
                    .filter(|p| p.secret && mask)
                    .map(|p| p.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        Some(renderer::render(
            &tpl,
            &self.values,
            &self.enabled,
            &secrets,
        ))
    }
    pub fn preview_text(&self) -> String {
        match (self.current_command(), self.render(true)) {
            (Some((_, c)), Some(r)) => preview::preview(c, &r),
            _ => "没有可用命令".into(),
        }
    }
    pub fn confirm(&mut self) {
        if let Some(r) = self.render(false) {
            if r.missing.is_empty() {
                self.output = Some(r.text);
                self.should_quit = true;
            } else {
                self.error = Some(format!("缺失参数：{}", r.missing.join(", ")));
            }
        }
    }
}
fn step(i: usize, n: usize, d: isize) -> usize {
    if n == 0 {
        0
    } else {
        ((i as isize + d).rem_euclid(n as isize)) as usize
    }
}
fn cycle_choice(values: &mut HashMap<String, String>, name: &str, choices: &[String]) {
    let cur = values.get(name);
    let pos = cur
        .and_then(|v| choices.iter().position(|c| c == v))
        .unwrap_or(choices.len() - 1);
    values.insert(name.to_string(), choices[(pos + 1) % choices.len()].clone());
}
