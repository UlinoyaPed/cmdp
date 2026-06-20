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
}

#[derive(Debug, Clone)]
pub enum FormItem {
    Param {
        name: String,
        label: String,
        value: String,
        placeholder: Option<String>,
        help: Option<String>,
        secret: bool,
        choices: Vec<String>,
        required: bool,
    },
    Option {
        id: String,
        label: String,
        enabled: bool,
    },
}

pub struct App {
    pub config: Config,
    pub category_idx: usize,
    pub command_idx: usize,
    pub form_idx: usize,
    pub focus: Focus,
    pub editing: bool,
    pub search_editing: bool,
    pub search_query: String,
    pub edit_buffer: String,
    pub edit_cursor: usize,
    pub values: HashMap<String, String>,
    pub enabled: HashSet<String>,
    pub should_quit: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub danger_confirmation: Option<String>,
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
            search_editing: false,
            search_query: String::new(),
            edit_buffer: String::new(),
            edit_cursor: 0,
            values: HashMap::new(),
            enabled: HashSet::new(),
            should_quit: false,
            output: None,
            error: None,
            danger_confirmation: None,
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
                self.editing = false;
                self.edit_cursor = 0;
                self.search_editing = false;
                self.search_query.clear();
                self.reset_form();
                self.error = None;
                self.danger_confirmation = None;
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

    pub fn visible_commands(&self) -> Vec<(&String, &Command)> {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            return self.commands_in_category();
        }

        self.config
            .commands
            .iter()
            .filter(|(id, cmd)| self.matches_search(id, cmd, &query))
            .collect()
    }

    pub fn current_command(&self) -> Option<(&String, &Command)> {
        self.visible_commands()
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

    pub fn form_items(&self) -> Vec<FormItem> {
        let Some((_, cmd)) = self.current_command() else {
            return Vec::new();
        };
        let Ok(parsed) = parser::parse_template(&cmd.template) else {
            return Vec::new();
        };
        let usage = parser::analyze_template(&parsed);
        let mut items = Vec::new();
        let mut shown_params = HashSet::new();

        for name in ordered_param_names(&usage.required_params, cmd) {
            push_param_item(
                &mut items,
                &mut shown_params,
                cmd,
                &self.values,
                &name,
                true,
            );
        }

        for optional in usage.optional {
            let option = cmd.options.iter().find(|o| o.id == optional.id);
            let label = option
                .and_then(|o| o.label.clone())
                .unwrap_or_else(|| optional.id.clone());
            let enabled = self.enabled.contains(&optional.id);
            items.push(FormItem::Option {
                id: optional.id.clone(),
                label,
                enabled,
            });

            if enabled {
                for name in ordered_param_names(&optional.params, cmd) {
                    push_param_item(
                        &mut items,
                        &mut shown_params,
                        cmd,
                        &self.values,
                        &name,
                        false,
                    );
                }
            }
        }

        items
    }

    pub fn form_len(&self) -> usize {
        self.form_items().len()
    }

    pub fn next_focus(&mut self, rev: bool) {
        self.error = None;
        self.focus = match (self.focus, rev) {
            (Focus::Categories, false) => Focus::Commands,
            (Focus::Commands, false) => Focus::Form,
            (Focus::Form, false) => Focus::Categories,
            (Focus::Categories, true) => Focus::Form,
            (Focus::Commands, true) => Focus::Categories,
            (Focus::Form, true) => Focus::Commands,
        };
        self.clamp_form();
    }

    pub fn select_category(&mut self, idx: usize) {
        if idx < self.category_ids().len() {
            self.error = None;
            self.danger_confirmation = None;
            self.focus = Focus::Categories;
            self.category_idx = idx;
            self.command_idx = 0;
            self.search_editing = false;
            self.search_query.clear();
            self.reset_form();
        }
    }

    pub fn select_command(&mut self, idx: usize) {
        if idx < self.visible_commands().len() {
            self.error = None;
            self.danger_confirmation = None;
            self.focus = Focus::Commands;
            self.search_editing = false;
            self.command_idx = idx;
            self.sync_category_to_current_command();
            self.reset_form();
        }
    }

    pub fn select_form_item(&mut self, idx: usize, activate: bool) {
        if idx < self.form_len() {
            self.error = None;
            self.danger_confirmation = None;
            self.focus = Focus::Form;
            self.form_idx = idx;
            if activate {
                self.activate();
            }
        }
    }

    pub fn move_sel(&mut self, down: bool) {
        self.error = None;
        self.danger_confirmation = None;
        let delta = if down { 1isize } else { -1 };
        match self.focus {
            Focus::Categories => {
                let n = self.category_ids().len();
                self.category_idx = step(self.category_idx, n, delta);
                self.command_idx = 0;
                self.search_editing = false;
                self.search_query.clear();
                self.reset_form();
            }
            Focus::Commands => {
                let n = self.visible_commands().len();
                self.command_idx = step(self.command_idx, n, delta);
                self.sync_category_to_current_command();
                self.reset_form();
            }
            Focus::Form => {
                self.form_idx = step(self.form_idx, self.form_len(), delta);
            }
        }
    }

    pub fn activate(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.search_editing = false;
        match self.focus {
            Focus::Categories => self.focus = Focus::Commands,
            Focus::Commands => self.focus = Focus::Form,
            Focus::Form => match self.form_items().get(self.form_idx).cloned() {
                Some(FormItem::Param { name, choices, .. }) if choices.is_empty() => {
                    self.editing = true;
                    self.edit_buffer = self.values.get(&name).cloned().unwrap_or_default();
                    self.edit_cursor = self.edit_buffer.chars().count();
                }
                Some(FormItem::Param { name, choices, .. }) => {
                    cycle_choice(&mut self.values, &name, &choices);
                }
                Some(FormItem::Option { id, .. }) => {
                    self.toggle_option(&id);
                }
                None => {}
            },
        }
    }

    pub fn toggle(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.search_editing = false;
        match self.form_items().get(self.form_idx).cloned() {
            Some(FormItem::Option { id, .. }) if self.focus == Focus::Form => {
                self.toggle_option(&id);
            }
            Some(FormItem::Param { name, choices, .. })
                if self.focus == Focus::Form && !choices.is_empty() =>
            {
                cycle_choice(&mut self.values, &name, &choices);
            }
            _ => {}
        }
        self.clamp_form();
    }

    pub fn commit_edit(&mut self) {
        if let Some(FormItem::Param { name, .. }) = self.form_items().get(self.form_idx).cloned() {
            self.values.insert(name, self.edit_buffer.clone());
        }
        self.editing = false;
        self.edit_cursor = 0;
        self.error = None;
        self.danger_confirmation = None;
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_cursor = 0;
    }

    pub fn insert_edit_char(&mut self, c: char) {
        self.clamp_edit_cursor();
        let idx = byte_index(&self.edit_buffer, self.edit_cursor);
        self.edit_buffer.insert(idx, c);
        self.edit_cursor += 1;
    }

    pub fn backspace_edit_char(&mut self) {
        self.clamp_edit_cursor();
        if self.edit_cursor == 0 {
            return;
        }
        let start = byte_index(&self.edit_buffer, self.edit_cursor - 1);
        let end = byte_index(&self.edit_buffer, self.edit_cursor);
        self.edit_buffer.replace_range(start..end, "");
        self.edit_cursor -= 1;
    }

    pub fn delete_edit_char(&mut self) {
        self.clamp_edit_cursor();
        let len = self.edit_buffer.chars().count();
        if self.edit_cursor >= len {
            return;
        }
        let start = byte_index(&self.edit_buffer, self.edit_cursor);
        let end = byte_index(&self.edit_buffer, self.edit_cursor + 1);
        self.edit_buffer.replace_range(start..end, "");
    }

    pub fn move_edit_cursor(&mut self, right: bool) {
        let len = self.edit_buffer.chars().count();
        self.edit_cursor = if right {
            (self.edit_cursor + 1).min(len)
        } else {
            self.edit_cursor.saturating_sub(1)
        };
    }

    pub fn move_edit_cursor_to_start(&mut self) {
        self.edit_cursor = 0;
    }

    pub fn move_edit_cursor_to_end(&mut self) {
        self.edit_cursor = self.edit_buffer.chars().count();
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
        if self.config.commands.is_empty() {
            return "没有可用命令\n请在 ~/.config/cmdp/ 添加 .toml 配置，或在当前项目创建 .cmdp.toml"
                .into();
        }
        match (self.current_command(), self.render(true)) {
            (Some((_, c)), Some(r)) => preview::preview(c, &r),
            _ => "没有可用命令".into(),
        }
    }

    pub fn begin_search(&mut self) {
        self.error = None;
        self.focus = Focus::Commands;
        self.search_editing = true;
        self.command_idx = 0;
        self.sync_category_to_current_command();
        self.reset_form();
    }

    pub fn push_search_char(&mut self, c: char) {
        self.error = None;
        self.danger_confirmation = None;
        self.search_query.push(c);
        self.search_changed();
    }

    pub fn pop_search_char(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.search_query.pop();
        self.search_changed();
    }

    pub fn clear_search(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.search_editing = false;
        if !self.search_query.is_empty() {
            self.search_query.clear();
            self.command_idx = 0;
            self.reset_form();
        }
    }

    pub fn finish_search(&mut self) {
        self.search_editing = false;
    }

    pub fn search_active(&self) -> bool {
        self.search_editing || !self.search_query.is_empty()
    }

    pub fn confirm(&mut self) {
        let Some(rendered) = self.render(false) else {
            return;
        };
        if !rendered.missing.is_empty() {
            self.danger_confirmation = None;
            self.error = Some(format!("缺失参数：{}", rendered.missing.join(", ")));
            return;
        }

        if self
            .current_command()
            .is_some_and(|(_, command)| command.danger)
            && self.danger_confirmation.as_deref() != Some(rendered.text.as_str())
        {
            self.danger_confirmation = Some(rendered.text);
            self.error = Some("危险命令：再次 Ctrl+y 或点击执行确认".to_string());
            return;
        }

        self.output = Some(rendered.text);
        self.should_quit = true;
    }

    fn toggle_option(&mut self, id: &str) {
        if !self.enabled.remove(id) {
            self.enabled.insert(id.to_string());
        }
    }

    fn search_changed(&mut self) {
        self.command_idx = 0;
        self.sync_category_to_current_command();
        self.reset_form();
    }

    fn sync_category_to_current_command(&mut self) {
        let category = self.current_command().map(|(_, cmd)| cmd.category.clone());
        if let Some(category) = category
            && let Some(idx) = self
                .category_ids()
                .iter()
                .position(|id| id.as_str() == category)
        {
            self.category_idx = idx;
        }
    }

    fn matches_search(&self, id: &str, cmd: &Command, query: &str) -> bool {
        if command_id_matches_query(id, query) {
            return true;
        }

        let category_alias = self
            .config
            .categories
            .get(&cmd.category)
            .and_then(|category| category.alias.as_deref())
            .unwrap_or_default();
        let haystack = format!(
            "{} {} {} {} {} {}",
            id,
            cmd.title.as_deref().unwrap_or_default(),
            cmd.description.as_deref().unwrap_or_default(),
            cmd.category,
            category_alias,
            cmd.source.label()
        )
        .to_lowercase();
        haystack.contains(query)
    }

    fn clamp_form(&mut self) {
        let len = self.form_len();
        if len == 0 {
            self.form_idx = 0;
        } else if self.form_idx >= len {
            self.form_idx = len - 1;
        }
    }

    fn clamp_edit_cursor(&mut self) {
        self.edit_cursor = self.edit_cursor.min(self.edit_buffer.chars().count());
    }
}

fn push_param_item(
    items: &mut Vec<FormItem>,
    shown: &mut HashSet<String>,
    cmd: &Command,
    values: &HashMap<String, String>,
    name: &str,
    required: bool,
) {
    if !shown.insert(name.to_string()) {
        return;
    }
    let param = cmd.params.iter().find(|p| p.name == name);
    let value = values
        .get(name)
        .cloned()
        .or_else(|| param.and_then(|p| p.default.clone()))
        .unwrap_or_default();
    items.push(FormItem::Param {
        name: name.to_string(),
        label: param
            .and_then(|p| p.label.clone())
            .unwrap_or_else(|| name.to_string()),
        value,
        placeholder: param.and_then(|p| p.placeholder.clone()),
        help: param.and_then(|p| p.help.clone()),
        secret: param.is_some_and(|p| p.secret),
        choices: param.and_then(|p| p.choices.clone()).unwrap_or_default(),
        required,
    });
}

fn ordered_param_names(names: &[String], cmd: &Command) -> Vec<String> {
    let mut ordered = Vec::new();
    for param in &cmd.params {
        if names.iter().any(|name| name == &param.name) {
            ordered.push(param.name.clone());
        }
    }
    for name in names {
        if !ordered.iter().any(|existing| existing == name) {
            ordered.push(name.clone());
        }
    }
    ordered
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

fn byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .map(|(idx, _)| idx)
        .nth(char_index)
        .unwrap_or(value.len())
}

fn command_id_matches_query(id: &str, query: &str) -> bool {
    let id = normalize_command_id_fuzzy_text(id);
    let query = normalize_command_id_fuzzy_text(query);
    !query.is_empty() && fuzzy_matches(&id, &query)
}

fn normalize_command_id_fuzzy_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn fuzzy_matches(haystack: &str, needle: &str) -> bool {
    let mut needle = needle.chars();
    let Some(mut expected) = needle.next() else {
        return false;
    };

    for ch in haystack.chars() {
        if ch == expected {
            let Some(next) = needle.next() else {
                return true;
            };
            expected = next;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn search_filters_commands_across_categories() {
        let mut app = App::new(test_config());

        app.begin_search();
        for ch in "cargo".chars() {
            app.push_search_char(ch);
        }

        let command_ids: Vec<_> = app
            .visible_commands()
            .into_iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(command_ids, vec!["cargo_check"]);
        assert_eq!(app.current_category_id().map(String::as_str), Some("dev"));
    }

    #[test]
    fn search_command_id_supports_fuzzy_matching_with_spaces() {
        let mut app = App::new(test_config());

        app.begin_search();
        for ch in "cargo check".chars() {
            app.push_search_char(ch);
        }

        let command_ids: Vec<_> = app
            .visible_commands()
            .into_iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(command_ids, vec!["cargo_check"]);
    }

    #[test]
    fn search_command_id_supports_fuzzy_matching() {
        let mut app = App::new(test_config());

        app.begin_search();
        for ch in "cgchk".chars() {
            app.push_search_char(ch);
        }

        let command_ids: Vec<_> = app
            .visible_commands()
            .into_iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(command_ids, vec!["cargo_check"]);
    }

    #[test]
    fn clear_search_returns_to_selected_category_commands() {
        let mut app = App::new(test_config());
        app.begin_search();
        for ch in "run".chars() {
            app.push_search_char(ch);
        }

        app.clear_search();

        assert!(!app.search_active());
        assert_eq!(
            app.current_category_id().map(String::as_str),
            Some("project")
        );
        let command_ids: Vec<_> = app
            .visible_commands()
            .into_iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(command_ids, vec!["run"]);
    }

    #[test]
    fn dangerous_commands_require_second_confirmation() {
        let mut app = App::new(test_config());
        app.select_command(1);

        app.confirm();

        assert!(!app.should_quit);
        assert!(app.output.is_none());
        assert!(app.danger_confirmation.is_some());
        assert!(
            app.error
                .as_deref()
                .is_some_and(|error| error.contains("危险命令"))
        );

        app.confirm();

        assert!(app.should_quit);
        assert_eq!(app.output.as_deref(), Some("rm -rf ./target"));
    }

    #[test]
    fn parameter_editing_supports_cursor_movement_and_unicode() {
        let mut app = App::new(test_config());
        app.focus = Focus::Form;
        app.form_idx = 0;
        app.activate();

        for ch in "a中c".chars() {
            app.insert_edit_char(ch);
        }
        app.move_edit_cursor(false);
        app.insert_edit_char('b');
        assert_eq!(app.edit_buffer, "a中bc");

        app.backspace_edit_char();
        app.move_edit_cursor(false);
        app.delete_edit_char();
        app.commit_edit();

        assert_eq!(app.values.get("path").map(String::as_str), Some("ac"));
    }

    #[test]
    fn empty_config_preview_points_to_config_files() {
        let app = App::new(Config::default());

        let preview = app.preview_text();

        assert!(preview.contains("~/.config/cmdp/"));
        assert!(preview.contains(".cmdp.toml"));
    }

    fn test_config() -> Config {
        let mut categories = IndexMap::new();
        categories.insert(
            "file".to_string(),
            Category {
                alias: Some("文件管理".to_string()),
                source: Source::Global,
            },
        );
        categories.insert(
            "dev".to_string(),
            Category {
                alias: Some("开发工具".to_string()),
                source: Source::Global,
            },
        );
        categories.insert(
            "project".to_string(),
            Category {
                alias: Some("当前项目".to_string()),
                source: Source::Local,
            },
        );

        let mut commands = IndexMap::new();
        commands.insert(
            "find_large".to_string(),
            command("file", "查找大文件", "find <<{{path}}>>", Source::Global),
        );
        commands.insert(
            "clean_target".to_string(),
            dangerous_command("file", "删除 target", "rm -rf ./target", Source::Global),
        );
        commands.insert(
            "cargo_check".to_string(),
            command("dev", "Cargo Check", "cargo check", Source::Global),
        );
        commands.insert(
            "run".to_string(),
            command("project", "运行项目", "cargo run", Source::Local),
        );

        Config {
            categories,
            commands,
            sources: vec!["global:/tmp/commands.toml".to_string()],
        }
    }

    fn command(category: &str, title: &str, template: &str, source: Source) -> Command {
        Command {
            category: category.to_string(),
            title: Some(title.to_string()),
            description: None,
            danger: false,
            template: template.to_string(),
            params: Vec::new(),
            options: Vec::new(),
            source,
        }
    }

    fn dangerous_command(category: &str, title: &str, template: &str, source: Source) -> Command {
        let mut command = command(category, title, template, source);
        command.danger = true;
        command
    }
}
