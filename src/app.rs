use crate::{
    config,
    i18n::{Language, Texts},
    parser::{self, ParsedTemplate},
    preview, renderer, state,
    template::*,
};
use serde::Deserialize;

const SETTINGS_ITEM_COUNT: usize = 5;
const CONFIG_EDITOR_FIELD_COUNT: usize = 9;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

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

#[derive(Debug, Clone)]
pub struct FilePicker {
    pub param_name: String,
    pub dir: PathBuf,
    pub entries: Vec<FilePickerEntry>,
    pub selected: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FilePickerEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigEditor {
    pub draft: ConfigDraft,
    pub target: ConfigEditTarget,
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub edit_cursor: usize,
    pub template_property_editor: Option<TemplatePropertyEditor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigEditTarget {
    GlobalEditor,
    LocalProject(PathBuf),
}

#[derive(Debug, Clone)]
pub struct TemplatePropertyEditor {
    pub part_index: usize,
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub edit_cursor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplatePart {
    pub kind: TemplatePartKind,
    pub token: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplatePartKind {
    Required,
    Optional { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplatePropertyField {
    pub label: String,
    pub value: String,
    pub kind: TemplatePropertyFieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplatePropertyFieldKind {
    OptionLabel(String),
    OptionDefaultEnabled(String),
    ParamLabel(String),
    ParamDefault(String),
    ParamPlaceholder(String),
    ParamHelp(String),
    ParamSecret(String),
    ParamChoices(String),
}

impl TemplatePropertyFieldKind {
    fn is_bool(&self) -> bool {
        matches!(
            self,
            TemplatePropertyFieldKind::OptionDefaultEnabled(_)
                | TemplatePropertyFieldKind::ParamSecret(_)
        )
    }
}

#[derive(Debug, Clone)]
pub struct ConfigDraft {
    pub command_id: String,
    pub category_id: String,
    pub category_alias: String,
    pub title: String,
    pub description: String,
    pub danger: String,
    pub template: String,
    pub params: String,
    pub options: String,
}

#[derive(Deserialize)]
struct ParamsSpec {
    params: Vec<Param>,
}

#[derive(Deserialize)]
struct OptionsSpec {
    options: Vec<OptionDef>,
}

#[derive(Deserialize)]
struct ChoicesSpec {
    choices: Vec<String>,
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
    pub preview_scroll: u16,
    pub history_cleared: bool,
    pub danger_confirmation: Option<String>,
    pub show_help: bool,
    pub show_settings: bool,
    pub settings_idx: usize,
    pub config_editor: Option<ConfigEditor>,
    pub file_picker: Option<FilePicker>,
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
            preview_scroll: 0,
            history_cleared: false,
            danger_confirmation: None,
            show_help: false,
            show_settings: false,
            settings_idx: 0,
            config_editor: None,
            file_picker: None,
        };
        app.reset_form();
        app.restore_last_selection();
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
                self.show_settings = false;
                self.config_editor = None;
                self.file_picker = None;
                self.error = None;
                self.reset_preview_scroll();
                self.reset_form();
                self.restore_last_selection();
                self.danger_confirmation = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    pub fn texts(&self) -> &'static Texts {
        self.config.settings.language.texts()
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
        self.reset_form_to_defaults();
        self.restore_current_input();
        self.reset_preview_scroll();
    }

    pub fn reset_current_form_to_defaults(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.editing = false;
        self.edit_buffer.clear();
        self.edit_cursor = 0;
        self.reset_form_to_defaults();
        self.remove_current_input_record();
        self.reset_preview_scroll();
    }

    fn reset_form_to_defaults(&mut self) {
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

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn close_help(&mut self) {
        self.show_help = false;
    }

    pub fn toggle_settings(&mut self) {
        if self.show_settings {
            self.close_settings();
        } else {
            self.open_settings();
        }
    }

    pub fn open_settings(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.show_help = false;
        self.file_picker = None;
        self.config_editor = None;
        self.editing = false;
        self.search_editing = false;
        self.show_settings = true;
        self.settings_idx = self.settings_idx.min(SETTINGS_ITEM_COUNT - 1);
    }

    pub fn close_settings(&mut self) {
        self.show_settings = false;
    }

    pub fn move_settings(&mut self, down: bool) {
        self.settings_idx = step(
            self.settings_idx,
            SETTINGS_ITEM_COUNT,
            if down { 1 } else { -1 },
        );
    }

    pub fn select_setting(&mut self, idx: usize, adjust: bool) {
        if idx < SETTINGS_ITEM_COUNT {
            self.settings_idx = idx;
            if adjust {
                self.adjust_setting(true);
            }
        }
    }

    pub fn adjust_setting(&mut self, forward: bool) {
        if self.settings_idx == 4 {
            self.clear_history();
        } else {
            adjust_setting_value(&mut self.config.settings, self.settings_idx, forward);
            self.persist_settings();
        }
    }

    pub fn clear_history(&mut self) {
        match state::clear() {
            Ok(()) => {
                self.history_cleared = true;
                self.error = None;
            }
            Err(error) => {
                self.error = Some(format!(
                    "{}{error}",
                    self.texts().clear_history_failed_prefix
                ));
            }
        }
    }

    pub fn open_file_picker(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.show_settings = false;
        self.config_editor = None;
        let Some(FormItem::Param { name, choices, .. }) =
            self.form_items().get(self.form_idx).cloned()
        else {
            self.error = Some(self.texts().not_text_param_file_picker.to_string());
            return;
        };
        if !choices.is_empty() {
            self.error = Some(self.texts().not_text_param_file_picker.to_string());
            return;
        }

        let dir = self.file_picker_start_dir(&name);
        self.focus = Focus::Form;
        self.file_picker = Some(load_file_picker(name, dir, self.texts()));
    }

    pub fn close_file_picker(&mut self) {
        self.file_picker = None;
    }

    pub fn move_file_picker(&mut self, down: bool) {
        let Some(picker) = self.file_picker.as_mut() else {
            return;
        };
        picker.selected = step(
            picker.selected,
            picker.entries.len(),
            if down { 1 } else { -1 },
        );
    }

    pub fn file_picker_parent(&mut self) {
        let Some(picker) = self.file_picker.as_ref() else {
            return;
        };
        let param_name = picker.param_name.clone();
        let parent = picker.dir.parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            self.file_picker = Some(load_file_picker(param_name, parent, self.texts()));
        }
    }

    pub fn file_picker_activate(&mut self) {
        let Some(entry) = self
            .file_picker
            .as_ref()
            .and_then(|picker| picker.entries.get(picker.selected).cloned())
        else {
            return;
        };

        if entry.is_dir {
            let param_name = self
                .file_picker
                .as_ref()
                .map(|picker| picker.param_name.clone())
                .unwrap_or_default();
            self.file_picker = Some(load_file_picker(param_name, entry.path, self.texts()));
        } else {
            self.file_picker_select();
        }
    }

    pub fn file_picker_select(&mut self) {
        let Some((param_name, path)) = self.file_picker.as_ref().and_then(|picker| {
            picker
                .entries
                .get(picker.selected)
                .map(|entry| (picker.param_name.clone(), entry.path.clone()))
        }) else {
            return;
        };
        self.values.insert(param_name, display_path(&path));
        self.file_picker = None;
        self.error = None;
        self.danger_confirmation = None;
        self.persist_current_input();
    }

    pub fn select_file_picker_entry(&mut self, idx: usize, activate: bool) {
        let Some(picker) = self.file_picker.as_mut() else {
            return;
        };
        if idx >= picker.entries.len() {
            return;
        }
        picker.selected = idx;
        if activate {
            self.file_picker_activate();
        }
    }

    pub fn open_config_editor(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.show_help = false;
        self.show_settings = false;
        self.file_picker = None;
        self.editing = false;
        self.search_editing = false;
        let draft = self.current_config_draft();
        let target = self.current_config_edit_target();
        self.config_editor = Some(ConfigEditor {
            draft,
            target,
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            edit_cursor: 0,
            template_property_editor: None,
        });
    }

    pub fn close_config_editor(&mut self) {
        self.config_editor = None;
    }

    pub fn config_editor_item_count(&self) -> usize {
        self.config_editor
            .as_ref()
            .map(config_editor_item_count)
            .unwrap_or(0)
    }

    pub fn config_editor_template_parts(&self) -> Vec<TemplatePart> {
        self.config_editor
            .as_ref()
            .and_then(|editor| template_parts(&editor.draft.template).ok())
            .unwrap_or_default()
    }

    pub fn config_editor_template_part_labels(&self) -> Vec<String> {
        let Some(editor) = self.config_editor.as_ref() else {
            return Vec::new();
        };
        let Ok(parts) = template_parts(&editor.draft.template) else {
            return Vec::new();
        };
        parts
            .iter()
            .map(|part| template_part_labels(&editor.draft, part).unwrap_or_default())
            .collect()
    }

    pub fn config_editor_field_preview(&self, idx: usize) -> Option<String> {
        let editor = self.config_editor.as_ref()?;
        match idx {
            7 => params_label_preview(&editor.draft.params).ok(),
            8 => options_label_preview(&editor.draft.options).ok(),
            _ => None,
        }
    }

    pub fn reset_config_editor_to_new_command(&mut self) {
        let draft = self.new_config_draft();
        if let Some(editor) = self.config_editor.as_mut() {
            editor.draft = draft;
            editor.target = ConfigEditTarget::GlobalEditor;
            editor.selected = 0;
            editor.editing = false;
            editor.edit_buffer.clear();
            editor.edit_cursor = 0;
            editor.template_property_editor = None;
        }
    }

    pub fn move_config_editor(&mut self, down: bool) {
        let Some(item_count) = self.config_editor.as_ref().map(config_editor_item_count) else {
            return;
        };
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        editor.selected = step(editor.selected, item_count, if down { 1 } else { -1 });
    }

    pub fn select_config_editor_field(&mut self, idx: usize, activate: bool) {
        if idx >= self.config_editor_item_count() {
            return;
        }
        if self
            .config_editor
            .as_ref()
            .is_some_and(|editor| editor.editing)
        {
            self.commit_config_editor_edit();
        }
        if let Some(editor) = self.config_editor.as_mut() {
            editor.selected = idx;
        }
        if activate {
            self.begin_config_editor_edit();
        }
    }

    pub fn begin_config_editor_edit(&mut self) {
        if self
            .config_editor
            .as_ref()
            .is_some_and(|editor| editor.selected >= CONFIG_EDITOR_FIELD_COUNT)
        {
            let part_index = self
                .config_editor
                .as_ref()
                .map(|editor| editor.selected - CONFIG_EDITOR_FIELD_COUNT)
                .unwrap_or_default();
            self.open_config_template_property_editor(part_index);
            return;
        }

        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        editor.editing = true;
        editor.edit_buffer = editor.draft.field(editor.selected).to_string();
        editor.edit_cursor = editor.edit_buffer.chars().count();
    }

    pub fn commit_config_editor_edit(&mut self) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        editor
            .draft
            .set_field(editor.selected, editor.edit_buffer.clone());
        editor.editing = false;
        editor.edit_cursor = 0;
        clamp_config_editor_selection(editor);
    }

    pub fn cancel_config_editor_edit(&mut self) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        editor.editing = false;
        editor.edit_cursor = 0;
    }

    pub fn insert_config_editor_char(&mut self, c: char) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        clamp_text_cursor(&mut editor.edit_cursor, &editor.edit_buffer);
        let idx = byte_index(&editor.edit_buffer, editor.edit_cursor);
        editor.edit_buffer.insert(idx, c);
        editor.edit_cursor += 1;
    }

    pub fn insert_config_editor_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.insert_config_editor_char(ch);
        }
    }

    pub fn backspace_config_editor_char(&mut self) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        clamp_text_cursor(&mut editor.edit_cursor, &editor.edit_buffer);
        if editor.edit_cursor == 0 {
            return;
        }
        let start = byte_index(&editor.edit_buffer, editor.edit_cursor - 1);
        let end = byte_index(&editor.edit_buffer, editor.edit_cursor);
        editor.edit_buffer.replace_range(start..end, "");
        editor.edit_cursor -= 1;
    }

    pub fn delete_config_editor_char(&mut self) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        clamp_text_cursor(&mut editor.edit_cursor, &editor.edit_buffer);
        let len = editor.edit_buffer.chars().count();
        if editor.edit_cursor >= len {
            return;
        }
        let start = byte_index(&editor.edit_buffer, editor.edit_cursor);
        let end = byte_index(&editor.edit_buffer, editor.edit_cursor + 1);
        editor.edit_buffer.replace_range(start..end, "");
    }

    pub fn move_config_editor_cursor(&mut self, right: bool) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        let len = editor.edit_buffer.chars().count();
        editor.edit_cursor = if right {
            (editor.edit_cursor + 1).min(len)
        } else {
            editor.edit_cursor.saturating_sub(1)
        };
    }

    pub fn move_config_editor_cursor_to_start(&mut self) {
        if let Some(editor) = self.config_editor.as_mut() {
            editor.edit_cursor = 0;
        }
    }

    pub fn move_config_editor_cursor_to_end(&mut self) {
        if let Some(editor) = self.config_editor.as_mut() {
            editor.edit_cursor = editor.edit_buffer.chars().count();
        }
    }

    pub fn config_template_property_is_open(&self) -> bool {
        self.config_editor
            .as_ref()
            .is_some_and(|editor| editor.template_property_editor.is_some())
    }

    pub fn config_template_property_fields(&self) -> Vec<TemplatePropertyField> {
        let Some(editor) = self.config_editor.as_ref() else {
            return Vec::new();
        };
        let Some(property_editor) = editor.template_property_editor.as_ref() else {
            return Vec::new();
        };
        let Ok(parts) = template_parts(&editor.draft.template) else {
            return Vec::new();
        };
        let Some(part) = parts.get(property_editor.part_index) else {
            return Vec::new();
        };
        template_property_fields(&editor.draft, part).unwrap_or_default()
    }

    pub fn config_template_property_part_label(&self) -> Option<String> {
        let editor = self.config_editor.as_ref()?;
        let property_editor = editor.template_property_editor.as_ref()?;
        let parts = template_parts(&editor.draft.template).ok()?;
        let part = parts.get(property_editor.part_index)?;
        template_part_labels(&editor.draft, part)
            .ok()
            .filter(|label| !label.trim().is_empty())
            .or_else(|| Some(template_part_display(part)))
    }

    pub fn open_config_template_property_editor(&mut self, part_index: usize) {
        let Some(editor) = self.config_editor.as_mut() else {
            return;
        };
        let Ok(parts) = template_parts(&editor.draft.template) else {
            return;
        };
        if part_index >= parts.len() {
            return;
        }
        editor.editing = false;
        editor.edit_buffer.clear();
        editor.edit_cursor = 0;
        editor.template_property_editor = Some(TemplatePropertyEditor {
            part_index,
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            edit_cursor: 0,
        });
    }

    pub fn close_config_template_property_editor(&mut self) {
        if let Some(editor) = self.config_editor.as_mut() {
            editor.template_property_editor = None;
        }
    }

    pub fn move_config_template_property(&mut self, down: bool) {
        let field_count = self.config_template_property_fields().len();
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        property_editor.selected = step(
            property_editor.selected,
            field_count,
            if down { 1 } else { -1 },
        );
    }

    pub fn select_config_template_property(&mut self, idx: usize, activate: bool) {
        if idx >= self.config_template_property_fields().len() {
            return;
        }
        if self
            .config_editor
            .as_ref()
            .and_then(|editor| editor.template_property_editor.as_ref())
            .is_some_and(|property_editor| property_editor.editing)
        {
            self.commit_config_template_property_edit();
            if self
                .config_editor
                .as_ref()
                .and_then(|editor| editor.template_property_editor.as_ref())
                .is_some_and(|property_editor| property_editor.editing)
            {
                return;
            }
        }
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        property_editor.selected = idx;
        if activate {
            if self
                .config_template_property_fields()
                .get(idx)
                .is_some_and(|field| field.kind.is_bool())
            {
                self.toggle_config_template_property();
            } else {
                self.begin_config_template_property_edit();
            }
        }
    }

    pub fn begin_config_template_property_edit(&mut self) {
        let fields = self.config_template_property_fields();
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        let Some(field) = fields.get(property_editor.selected) else {
            return;
        };
        property_editor.editing = true;
        property_editor.edit_buffer = field.value.clone();
        property_editor.edit_cursor = property_editor.edit_buffer.chars().count();
    }

    pub fn commit_config_template_property_edit(&mut self) {
        let Some((kind, value)) = self.active_template_property_edit() else {
            return;
        };
        if let Some(editor) = self.config_editor.as_mut() {
            match set_template_property(&mut editor.draft, &kind, &value) {
                Ok(()) => {
                    if let Some(property_editor) = editor.template_property_editor.as_mut() {
                        property_editor.editing = false;
                        property_editor.edit_cursor = 0;
                    }
                    self.error = None;
                }
                Err(error) => {
                    self.error = Some(format!(
                        "{}{error}",
                        self.texts().config_editor_invalid_params_prefix
                    ));
                }
            }
        }
    }

    pub fn cancel_config_template_property_edit(&mut self) {
        if let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        {
            property_editor.editing = false;
            property_editor.edit_cursor = 0;
        }
    }

    pub fn toggle_config_template_property(&mut self) {
        let Some(field) = self
            .config_template_property_fields()
            .get(
                self.config_editor
                    .as_ref()
                    .and_then(|editor| editor.template_property_editor.as_ref())
                    .map(|property_editor| property_editor.selected)
                    .unwrap_or_default(),
            )
            .cloned()
        else {
            return;
        };
        if !field.kind.is_bool() {
            return;
        }
        let value = if parse_bool(&field.value).unwrap_or(false) {
            "false"
        } else {
            "true"
        };
        if let Some(editor) = self.config_editor.as_mut()
            && let Err(error) = set_template_property(&mut editor.draft, &field.kind, value)
        {
            self.error = Some(format!(
                "{}{error}",
                self.texts().config_editor_invalid_params_prefix
            ));
        } else {
            self.error = None;
        }
    }

    pub fn insert_config_template_property_char(&mut self, c: char) {
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        clamp_text_cursor(
            &mut property_editor.edit_cursor,
            &property_editor.edit_buffer,
        );
        let idx = byte_index(&property_editor.edit_buffer, property_editor.edit_cursor);
        property_editor.edit_buffer.insert(idx, c);
        property_editor.edit_cursor += 1;
    }

    pub fn insert_config_template_property_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.insert_config_template_property_char(ch);
        }
    }

    pub fn backspace_config_template_property_char(&mut self) {
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        clamp_text_cursor(
            &mut property_editor.edit_cursor,
            &property_editor.edit_buffer,
        );
        if property_editor.edit_cursor == 0 {
            return;
        }
        let start = byte_index(
            &property_editor.edit_buffer,
            property_editor.edit_cursor - 1,
        );
        let end = byte_index(&property_editor.edit_buffer, property_editor.edit_cursor);
        property_editor.edit_buffer.replace_range(start..end, "");
        property_editor.edit_cursor -= 1;
    }

    pub fn delete_config_template_property_char(&mut self) {
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        clamp_text_cursor(
            &mut property_editor.edit_cursor,
            &property_editor.edit_buffer,
        );
        let len = property_editor.edit_buffer.chars().count();
        if property_editor.edit_cursor >= len {
            return;
        }
        let start = byte_index(&property_editor.edit_buffer, property_editor.edit_cursor);
        let end = byte_index(
            &property_editor.edit_buffer,
            property_editor.edit_cursor + 1,
        );
        property_editor.edit_buffer.replace_range(start..end, "");
    }

    pub fn move_config_template_property_cursor(&mut self, right: bool) {
        let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        else {
            return;
        };
        let len = property_editor.edit_buffer.chars().count();
        property_editor.edit_cursor = if right {
            (property_editor.edit_cursor + 1).min(len)
        } else {
            property_editor.edit_cursor.saturating_sub(1)
        };
    }

    pub fn move_config_template_property_cursor_to_start(&mut self) {
        if let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        {
            property_editor.edit_cursor = 0;
        }
    }

    pub fn move_config_template_property_cursor_to_end(&mut self) {
        if let Some(property_editor) = self
            .config_editor
            .as_mut()
            .and_then(|editor| editor.template_property_editor.as_mut())
        {
            property_editor.edit_cursor = property_editor.edit_buffer.chars().count();
        }
    }

    pub fn save_config_editor(&mut self) {
        if self
            .config_editor
            .as_ref()
            .and_then(|editor| editor.template_property_editor.as_ref())
            .is_some_and(|property_editor| property_editor.editing)
        {
            self.commit_config_template_property_edit();
            if self
                .config_editor
                .as_ref()
                .and_then(|editor| editor.template_property_editor.as_ref())
                .is_some_and(|property_editor| property_editor.editing)
            {
                return;
            }
        }

        if self
            .config_editor
            .as_ref()
            .is_some_and(|editor| editor.editing)
        {
            self.commit_config_editor_edit();
        }

        let Some(editor) = self.config_editor.as_ref() else {
            return;
        };
        let edit = match editor.draft.to_command_edit() {
            Ok(edit) => edit,
            Err(error) => {
                self.error = Some(format!(
                    "{}{error}",
                    self.texts().config_editor_invalid_params_prefix
                ));
                return;
            }
        };
        let target = editor.target.clone();

        let save_result = match target {
            ConfigEditTarget::GlobalEditor => config::save_command_edit(&edit),
            ConfigEditTarget::LocalProject(path) => {
                config::save_command_edit_to_local_path(&path, &edit)
            }
        };

        if let Err(error) = save_result {
            self.error = Some(format!(
                "{}{error}",
                self.texts().config_editor_save_failed_prefix
            ));
            return;
        }

        self.reload();
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
            self.file_picker = None;
            self.reset_form();
            self.persist_selection();
        }
    }

    pub fn select_command(&mut self, idx: usize) {
        if idx < self.visible_commands().len() {
            self.error = None;
            self.danger_confirmation = None;
            self.focus = Focus::Commands;
            self.search_editing = false;
            self.file_picker = None;
            self.command_idx = idx;
            self.sync_category_to_current_command();
            self.reset_form();
            self.persist_selection();
        }
    }

    pub fn select_form_item(&mut self, idx: usize, activate: bool) {
        if idx < self.form_len() {
            self.error = None;
            self.danger_confirmation = None;
            self.focus = Focus::Form;
            self.form_idx = idx;
            self.file_picker = None;
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
                self.file_picker = None;
                self.reset_form();
                self.persist_selection();
            }
            Focus::Commands => {
                let n = self.visible_commands().len();
                self.command_idx = step(self.command_idx, n, delta);
                self.file_picker = None;
                self.sync_category_to_current_command();
                self.reset_form();
                self.persist_selection();
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
        self.file_picker = None;
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
                    self.persist_current_input();
                    self.reset_preview_scroll();
                }
                Some(FormItem::Option { id, .. }) => {
                    self.toggle_option(&id);
                    self.persist_current_input();
                    self.reset_preview_scroll();
                }
                None => {}
            },
        }
    }

    pub fn toggle(&mut self) {
        self.error = None;
        self.danger_confirmation = None;
        self.search_editing = false;
        self.file_picker = None;
        match self.form_items().get(self.form_idx).cloned() {
            Some(FormItem::Option { id, .. }) if self.focus == Focus::Form => {
                self.toggle_option(&id);
                self.persist_current_input();
                self.reset_preview_scroll();
            }
            Some(FormItem::Param { name, choices, .. })
                if self.focus == Focus::Form && !choices.is_empty() =>
            {
                cycle_choice(&mut self.values, &name, &choices);
                self.persist_current_input();
                self.reset_preview_scroll();
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
        self.persist_current_input();
        self.reset_preview_scroll();
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
            return self.texts().empty_config_preview.into();
        }
        match (self.current_command(), self.render(true)) {
            (Some((_, c)), Some(r)) => preview::preview(c, &r, self.texts()),
            _ => self.texts().no_available_command.into(),
        }
    }

    pub fn persist_exit_state(&mut self) {
        if !self.history_cleared {
            self.persist_selection();
        }
    }

    pub fn reset_preview_scroll(&mut self) {
        self.preview_scroll = 0;
    }

    pub fn scroll_preview(&mut self, down: bool, viewport_width: u16, viewport_height: u16) {
        let max_scroll = self.preview_scroll_max(viewport_width, viewport_height);
        let current = self.preview_scroll.min(max_scroll);
        self.preview_scroll = if down {
            current.saturating_add(1).min(max_scroll)
        } else {
            current.saturating_sub(1)
        };
    }

    pub fn preview_scroll_offset(&self, viewport_width: u16, viewport_height: u16) -> u16 {
        self.preview_scroll
            .min(self.preview_scroll_max(viewport_width, viewport_height))
    }

    pub fn preview_scroll_max(&self, viewport_width: u16, viewport_height: u16) -> u16 {
        let content_lines = self.preview_display_line_count(viewport_width);
        let visible_lines = viewport_height.max(1) as usize;
        content_lines
            .saturating_sub(visible_lines)
            .min(u16::MAX as usize) as u16
    }

    fn preview_display_line_count(&self, viewport_width: u16) -> usize {
        let width = viewport_width.max(1) as usize;
        let mut lines = 0;
        if let Some(error) = &self.error {
            lines += wrapped_text_line_count(error, width);
        }
        lines + wrapped_text_line_count(&self.preview_text(), width)
    }

    pub fn begin_search(&mut self) {
        self.error = None;
        self.focus = Focus::Commands;
        self.search_editing = true;
        self.show_settings = false;
        self.config_editor = None;
        self.file_picker = None;
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
            self.error = Some(format!(
                "{}{}",
                self.texts().missing_params_prefix,
                rendered.missing.join(", ")
            ));
            return;
        }

        if self
            .current_command()
            .is_some_and(|(_, command)| command.danger)
            && self.danger_confirmation.as_deref() != Some(rendered.text.as_str())
        {
            self.danger_confirmation = Some(rendered.text);
            self.error = Some(self.texts().danger_confirmation.to_string());
            return;
        }

        self.persist_current_input();
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

    fn restore_last_selection(&mut self) {
        if !self.config.settings.remember_last_selection {
            return;
        }

        match state::load() {
            Ok(Some(state)) => self.apply_selection_state(&state),
            Ok(None) => {}
            Err(error) => {
                self.error = Some(format!(
                    "{}{error}",
                    self.texts().read_last_selection_failed_prefix
                ));
            }
        }
    }

    fn persist_selection(&mut self) {
        if !self.config.settings.remember_last_selection {
            return;
        }

        let mut app_state = self.load_app_state_or_default();
        app_state.category_id = self.current_category_id().cloned();
        app_state.command_id = self.current_command().map(|(id, _)| id.clone());
        app_state.focus = Some(focus_state_id(self.focus).to_string());
        self.clamp_input_records(&mut app_state);
        if let Err(error) = state::save(&app_state) {
            self.error = Some(format!(
                "{}{error}",
                self.texts().save_last_selection_failed_prefix
            ));
        } else {
            self.history_cleared = false;
        }
    }

    fn restore_current_input(&mut self) {
        if !self.config.settings.remember_last_input {
            return;
        }

        match state::load() {
            Ok(Some(state)) => self.apply_current_input(&state),
            Ok(None) => {}
            Err(error) => {
                self.error = Some(format!(
                    "{}{error}",
                    self.texts().read_last_input_failed_prefix
                ));
            }
        }
    }

    fn persist_current_input(&mut self) {
        if !self.config.settings.remember_last_input {
            return;
        }

        let Some((command_id, command)) = self.current_command() else {
            return;
        };
        let command_id = command_id.clone();
        let remembered_params: HashSet<_> = command
            .params
            .iter()
            .filter(|param| !param.secret)
            .map(|param| param.name.clone())
            .collect();
        let mut app_state = self.load_app_state_or_default();
        let record = InputRecord {
            command_id: command_id.clone(),
            values: self
                .values
                .iter()
                .filter(|(name, _)| remembered_params.contains(name.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            enabled: sorted_enabled(&self.enabled),
        };
        app_state
            .input_records
            .retain(|record| record.command_id != command_id);
        app_state.input_records.insert(0, record);
        self.clamp_input_records(&mut app_state);
        if let Err(error) = state::save(&app_state) {
            self.error = Some(format!(
                "{}{error}",
                self.texts().save_last_input_failed_prefix
            ));
        } else {
            self.history_cleared = false;
        }
    }

    fn remove_current_input_record(&mut self) {
        if !self.config.settings.remember_last_input {
            return;
        }

        let Some((command_id, _)) = self.current_command() else {
            return;
        };
        let command_id = command_id.clone();
        let mut app_state = self.load_app_state_or_default();
        app_state
            .input_records
            .retain(|record| record.command_id != command_id);
        if let Err(error) = state::save(&app_state) {
            self.error = Some(format!(
                "{}{error}",
                self.texts().clear_last_input_failed_prefix
            ));
        }
    }

    fn load_app_state_or_default(&mut self) -> AppState {
        match state::load() {
            Ok(Some(state)) => state,
            Ok(None) => AppState::default(),
            Err(error) => {
                self.error = Some(format!("{}{error}", self.texts().read_state_failed_prefix));
                AppState::default()
            }
        }
    }

    fn apply_selection_state(&mut self, state: &AppState) {
        let mut restored_command = false;
        if let Some(command_id) = state.command_id.as_deref()
            && let Some((_, command)) = self.config.commands.get_key_value(command_id)
        {
            if let Some(category_idx) = self
                .category_ids()
                .iter()
                .position(|id| id.as_str() == command.category)
            {
                self.category_idx = category_idx;
            }
            if let Some(command_idx) = self
                .visible_commands()
                .iter()
                .position(|(id, _)| id.as_str() == command_id)
            {
                self.command_idx = command_idx;
                self.reset_form();
                restored_command = true;
            }
        }

        if !restored_command
            && let Some(category_id) = state.category_id.as_deref()
            && let Some(category_idx) = self
                .category_ids()
                .iter()
                .position(|id| id.as_str() == category_id)
        {
            self.category_idx = category_idx;
            self.command_idx = 0;
            self.reset_form();
        }
        self.apply_focus_state(state);
    }

    fn apply_focus_state(&mut self, state: &AppState) {
        if let Some(focus) = state.focus.as_deref().and_then(focus_from_state_id) {
            self.focus = focus;
        }
        self.clamp_form();
    }

    fn apply_current_input(&mut self, state: &AppState) {
        let Some((command_id, command)) = self.current_command() else {
            return;
        };
        let command_id = command_id.clone();
        let param_names: HashSet<_> = command
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let option_ids: HashSet<_> = command
            .options
            .iter()
            .map(|option| option.id.clone())
            .collect();
        let Some(record) = state
            .input_records
            .iter()
            .find(|record| record.command_id == command_id)
        else {
            return;
        };

        for (name, value) in &record.values {
            if param_names.contains(name) {
                self.values.insert(name.clone(), value.clone());
            }
        }

        self.enabled = record
            .enabled
            .iter()
            .filter(|id| option_ids.contains(id.as_str()))
            .cloned()
            .collect();
    }

    fn clamp_input_records(&self, state: &mut AppState) {
        let limit = self.config.settings.input_record_limit;
        state.input_records.truncate(limit);
    }

    fn persist_settings(&mut self) {
        if let Err(error) = config::save_settings(&self.config.settings) {
            self.error = Some(format!(
                "{}{error}",
                self.texts().save_settings_failed_prefix
            ));
        } else {
            self.error = None;
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

    fn file_picker_start_dir(&self, param_name: &str) -> PathBuf {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let Some(value) = self
            .values
            .get(param_name)
            .filter(|value| !value.is_empty())
        else {
            return current_dir;
        };
        let path = PathBuf::from(value);
        let path = if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        };
        if path.is_dir() {
            path
        } else if let Some(parent) = path.parent().filter(|parent| parent.is_dir()) {
            parent.to_path_buf()
        } else {
            current_dir
        }
    }

    fn current_config_draft(&self) -> ConfigDraft {
        if let Some((id, command)) = self.current_command() {
            let category_alias = self
                .config
                .categories
                .get(&command.category)
                .and_then(|category| category.alias.clone())
                .unwrap_or_else(|| command.category.clone());
            return ConfigDraft {
                command_id: id.clone(),
                category_id: command.category.clone(),
                category_alias,
                title: command.title.clone().unwrap_or_else(|| id.clone()),
                description: command.description.clone().unwrap_or_default(),
                danger: command.danger.to_string(),
                template: command.template.clone(),
                params: params_spec(&command.params),
                options: options_spec(&command.options),
            };
        }

        self.new_config_draft()
    }

    fn current_config_edit_target(&self) -> ConfigEditTarget {
        let source = self.current_command().map(|(_, command)| command.source);
        let local_path = std::env::current_dir()
            .ok()
            .and_then(|cwd| config::find_local(&cwd));
        config_edit_target(source, local_path)
    }

    fn active_template_property_edit(&self) -> Option<(TemplatePropertyFieldKind, String)> {
        let editor = self.config_editor.as_ref()?;
        let property_editor = editor.template_property_editor.as_ref()?;
        if !property_editor.editing {
            return None;
        }
        let field = self
            .config_template_property_fields()
            .get(property_editor.selected)?
            .clone();
        Some((field.kind, property_editor.edit_buffer.clone()))
    }

    fn new_config_draft(&self) -> ConfigDraft {
        let category_id = self
            .current_category_id()
            .cloned()
            .unwrap_or_else(|| "general".to_string());
        let category_alias = self
            .config
            .categories
            .get(&category_id)
            .and_then(|category| category.alias.clone())
            .unwrap_or_else(|| category_id.clone());
        ConfigDraft {
            command_id: unique_command_id(&self.config, "new_command"),
            category_id,
            category_alias,
            title: "新命令".to_string(),
            description: String::new(),
            danger: "false".to_string(),
            template: "echo <<{{value}}>>".to_string(),
            params: r#"[{ name = "value", label = "参数" }]"#.to_string(),
            options: "[]".to_string(),
        }
    }
}

fn load_file_picker(param_name: String, dir: PathBuf, texts: &'static Texts) -> FilePicker {
    let mut picker = FilePicker {
        param_name,
        dir,
        entries: Vec::new(),
        selected: 0,
        error: None,
    };
    match read_file_entries(&picker.dir, texts) {
        Ok(entries) => picker.entries = entries,
        Err(error) => picker.error = Some(error),
    }
    picker
}

fn read_file_entries(dir: &Path, texts: &'static Texts) -> Result<Vec<FilePickerEntry>, String> {
    let mut entries = Vec::new();
    entries.push(FilePickerEntry {
        name: ".".to_string(),
        path: dir.to_path_buf(),
        is_dir: true,
    });
    for entry in
        fs::read_dir(dir).map_err(|error| format!("{}{error}", texts.read_dir_failed_prefix))?
    {
        let entry =
            entry.map_err(|error| format!("{}{error}", texts.read_dir_entry_failed_prefix))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}{error}", texts.read_file_type_failed_prefix))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(FilePickerEntry {
            name,
            path,
            is_dir: file_type.is_dir(),
        });
    }
    entries[1..].sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

fn display_path(path: &Path) -> String {
    let cwd = std::env::current_dir().ok();
    if let Some(cwd) = cwd
        && let Ok(relative) = path.strip_prefix(&cwd)
    {
        if relative.as_os_str().is_empty() {
            return ".".to_string();
        }
        return relative.display().to_string();
    }
    path.display().to_string()
}

impl ConfigDraft {
    pub fn field(&self, idx: usize) -> &str {
        match idx {
            0 => &self.command_id,
            1 => &self.category_id,
            2 => &self.category_alias,
            3 => &self.title,
            4 => &self.description,
            5 => &self.danger,
            6 => &self.template,
            7 => &self.params,
            8 => &self.options,
            _ => "",
        }
    }

    fn set_field(&mut self, idx: usize, value: String) {
        match idx {
            0 => self.command_id = value,
            1 => self.category_id = value,
            2 => self.category_alias = value,
            3 => self.title = value,
            4 => self.description = value,
            5 => self.danger = value,
            6 => self.template = value,
            7 => self.params = value,
            8 => self.options = value,
            _ => {}
        }
    }

    fn to_command_edit(&self) -> Result<config::CommandEdit, String> {
        Ok(config::CommandEdit {
            command_id: self.command_id.trim().to_string(),
            category_id: self.category_id.trim().to_string(),
            category_alias: optional_string(&self.category_alias),
            title: optional_string(&self.title),
            description: optional_string(&self.description),
            danger: parse_bool(&self.danger)?,
            template: self.template.trim().to_string(),
            params: parse_params_spec(&self.params)?,
            options: parse_options_spec(&self.options)?,
        })
    }
}

fn params_spec(params: &[Param]) -> String {
    inline_array(params.iter().map(param_inline).collect())
}

fn parse_params_spec(spec: &str) -> Result<Vec<Param>, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(Vec::new());
    }
    if spec.starts_with('[') {
        return toml::from_str::<ParamsSpec>(&format!("params = {spec}"))
            .map(|wrapper| wrapper.params)
            .map_err(|error| error.to_string());
    }

    parse_legacy_params_spec(spec)
}

fn options_spec(options: &[OptionDef]) -> String {
    inline_array(options.iter().map(option_inline).collect())
}

fn parse_options_spec(spec: &str) -> Result<Vec<OptionDef>, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(Vec::new());
    }
    toml::from_str::<OptionsSpec>(&format!("options = {spec}"))
        .map(|wrapper| wrapper.options)
        .map_err(|error| error.to_string())
}

fn params_label_preview(spec: &str) -> Result<String, String> {
    Ok(parse_params_spec(spec)?
        .iter()
        .map(|param| display_label(param.label.as_deref(), &param.name))
        .collect::<Vec<_>>()
        .join(", "))
}

fn options_label_preview(spec: &str) -> Result<String, String> {
    Ok(parse_options_spec(spec)?
        .iter()
        .map(|option| display_label(option.label.as_deref(), &option.id))
        .collect::<Vec<_>>()
        .join(", "))
}

fn parse_legacy_params_spec(spec: &str) -> Result<Vec<Param>, String> {
    spec.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            let (name, label) = item
                .split_once(':')
                .map(|(name, label)| (name.trim(), label.trim()))
                .unwrap_or((item, ""));
            if name.is_empty() {
                return Err("empty parameter name".to_string());
            }
            Ok(Param {
                name: name.to_string(),
                label: optional_string(label),
                default: None,
                placeholder: None,
                help: None,
                secret: false,
                choices: None,
            })
        })
        .collect()
}

fn inline_array(items: Vec<String>) -> String {
    format!("[{}]", items.join(", "))
}

fn param_inline(param: &Param) -> String {
    let mut fields = vec![inline_field("name", &param.name)];
    push_optional_field(&mut fields, "label", param.label.as_deref());
    push_optional_field(&mut fields, "default", param.default.as_deref());
    push_optional_field(&mut fields, "placeholder", param.placeholder.as_deref());
    push_optional_field(&mut fields, "help", param.help.as_deref());
    fields.push(format!("secret = {}", param.secret));
    if let Some(choices) = &param.choices {
        fields.push(format!(
            "choices = [{}]",
            choices
                .iter()
                .map(|choice| toml_string(choice))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    format!("{{ {} }}", fields.join(", "))
}

fn option_inline(option: &OptionDef) -> String {
    let mut fields = vec![inline_field("id", &option.id)];
    push_optional_field(&mut fields, "label", option.label.as_deref());
    fields.push(format!("default_enabled = {}", option.default_enabled));
    format!("{{ {} }}", fields.join(", "))
}

fn inline_field(key: &str, value: &str) -> String {
    format!("{key} = {}", toml_string(value))
}

fn push_optional_field(fields: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        fields.push(inline_field(key, value));
    }
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "是" | "开" => Ok(true),
        "false" | "0" | "no" | "off" | "否" | "关" => Ok(false),
        value => Err(format!("invalid bool '{value}'")),
    }
}

fn optional_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn unique_command_id(config: &Config, base: &str) -> String {
    if !config.commands.contains_key(base) {
        return base.to_string();
    }
    for idx in 2.. {
        let candidate = format!("{base}_{idx}");
        if !config.commands.contains_key(&candidate) {
            return candidate;
        }
    }
    unreachable!()
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

fn sorted_enabled(enabled: &HashSet<String>) -> Vec<String> {
    let mut enabled: Vec<_> = enabled.iter().cloned().collect();
    enabled.sort();
    enabled
}

fn config_editor_item_count(editor: &ConfigEditor) -> usize {
    CONFIG_EDITOR_FIELD_COUNT
        + template_parts(&editor.draft.template).map_or(0, |parts| parts.len())
}

fn clamp_config_editor_selection(editor: &mut ConfigEditor) {
    let count = config_editor_item_count(editor);
    if count == 0 {
        editor.selected = 0;
    } else if editor.selected >= count {
        editor.selected = count - 1;
    }
}

fn template_parts(template: &str) -> Result<Vec<TemplatePart>, String> {
    parser::parse_template(template)?;

    let mut parts = Vec::new();
    let mut i = 0;
    let mut optional_count = 0;
    while i < template.len() {
        let rest = &template[i..];
        if rest.starts_with("<<") {
            let end = rest.find(">>").ok_or("unclosed required segment")?;
            let body = &rest[2..end];
            let params = placeholder_names(body)?;
            if !params.is_empty() {
                parts.push(TemplatePart {
                    kind: TemplatePartKind::Required,
                    token: format!("<<{body}>>"),
                    params,
                });
            }
            i += end + 2;
        } else if rest.starts_with("[[") {
            let end = rest.find("]]").ok_or("unclosed optional segment")?;
            let raw = &rest[2..end];
            let (id, body) = if let Some((id, body)) = raw
                .split_once(':')
                .filter(|(id, body)| parser::is_identifier(id.trim()) && !body.starts_with("//"))
            {
                (id.trim().to_string(), body)
            } else {
                (format!("option_{}", optional_count + 1), raw)
            };
            optional_count += 1;
            parts.push(TemplatePart {
                kind: TemplatePartKind::Optional { id },
                token: format!("[[{raw}]]"),
                params: placeholder_names(body)?,
            });
            i += end + 2;
        } else {
            let next = [rest.find("<<"), rest.find("[[")]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(rest.len());
            i += next.max(1);
        }
    }

    Ok(parts)
}

fn placeholder_names(value: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut i = 0;
    while i < value.len() {
        let rest = &value[i..];
        let Some(start) = rest.find("{{") else {
            break;
        };
        let placeholder = &rest[start + 2..];
        let end = placeholder.find("}}").ok_or("unclosed placeholder")?;
        let name = placeholder[..end].trim();
        if !parser::is_identifier(name) {
            return Err(format!("invalid placeholder name '{name}'"));
        }
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
        i += start + end + 4;
    }
    Ok(names)
}

fn template_property_fields(
    draft: &ConfigDraft,
    part: &TemplatePart,
) -> Result<Vec<TemplatePropertyField>, String> {
    let params = parse_params_spec(&draft.params)?;
    let options = parse_options_spec(&draft.options)?;
    let mut fields = Vec::new();

    if let TemplatePartKind::Optional { id } = &part.kind {
        let option = options.iter().find(|option| option.id == *id);
        fields.push(TemplatePropertyField {
            label: format!("{id}.label"),
            value: option
                .and_then(|option| option.label.clone())
                .unwrap_or_default(),
            kind: TemplatePropertyFieldKind::OptionLabel(id.clone()),
        });
        fields.push(TemplatePropertyField {
            label: format!("{id}.default_enabled"),
            value: option
                .map(|option| option.default_enabled)
                .unwrap_or_default()
                .to_string(),
            kind: TemplatePropertyFieldKind::OptionDefaultEnabled(id.clone()),
        });
    }

    for name in &part.params {
        let param = params.iter().find(|param| param.name == *name);
        fields.push(TemplatePropertyField {
            label: format!("{name}.label"),
            value: param
                .and_then(|param| param.label.clone())
                .unwrap_or_default(),
            kind: TemplatePropertyFieldKind::ParamLabel(name.clone()),
        });
        fields.push(TemplatePropertyField {
            label: format!("{name}.default"),
            value: param
                .and_then(|param| param.default.clone())
                .unwrap_or_default(),
            kind: TemplatePropertyFieldKind::ParamDefault(name.clone()),
        });
        fields.push(TemplatePropertyField {
            label: format!("{name}.placeholder"),
            value: param
                .and_then(|param| param.placeholder.clone())
                .unwrap_or_default(),
            kind: TemplatePropertyFieldKind::ParamPlaceholder(name.clone()),
        });
        fields.push(TemplatePropertyField {
            label: format!("{name}.help"),
            value: param
                .and_then(|param| param.help.clone())
                .unwrap_or_default(),
            kind: TemplatePropertyFieldKind::ParamHelp(name.clone()),
        });
        fields.push(TemplatePropertyField {
            label: format!("{name}.secret"),
            value: param
                .map(|param| param.secret)
                .unwrap_or_default()
                .to_string(),
            kind: TemplatePropertyFieldKind::ParamSecret(name.clone()),
        });
        fields.push(TemplatePropertyField {
            label: format!("{name}.choices"),
            value: param
                .and_then(|param| param.choices.clone())
                .map(|choices| choices.join(", "))
                .unwrap_or_default(),
            kind: TemplatePropertyFieldKind::ParamChoices(name.clone()),
        });
    }

    Ok(fields)
}

fn template_part_labels(draft: &ConfigDraft, part: &TemplatePart) -> Result<String, String> {
    let params = parse_params_spec(&draft.params)?;
    let options = parse_options_spec(&draft.options)?;
    let mut labels = Vec::new();

    if let TemplatePartKind::Optional { id } = &part.kind {
        let option_label = options
            .iter()
            .find(|option| option.id == *id)
            .and_then(|option| option.label.as_deref())
            .filter(|label| !label.is_empty());
        labels.push(display_label(option_label, id));
    }

    for name in &part.params {
        let param_label = params
            .iter()
            .find(|param| param.name == *name)
            .and_then(|param| param.label.as_deref())
            .filter(|label| !label.is_empty());
        labels.push(display_label(param_label, name));
    }

    Ok(labels.join("  "))
}

fn display_label(label: Option<&str>, fallback: &str) -> String {
    label
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn set_template_property(
    draft: &mut ConfigDraft,
    kind: &TemplatePropertyFieldKind,
    value: &str,
) -> Result<(), String> {
    match kind {
        TemplatePropertyFieldKind::OptionLabel(id) => {
            let mut options = parse_options_spec(&draft.options)?;
            let option = ensure_option(&mut options, id);
            option.label = optional_string(value);
            draft.options = options_spec(&options);
        }
        TemplatePropertyFieldKind::OptionDefaultEnabled(id) => {
            let mut options = parse_options_spec(&draft.options)?;
            let option = ensure_option(&mut options, id);
            option.default_enabled = parse_bool(value)?;
            draft.options = options_spec(&options);
        }
        TemplatePropertyFieldKind::ParamLabel(name) => {
            update_param(draft, name, |param| param.label = optional_string(value))?;
        }
        TemplatePropertyFieldKind::ParamDefault(name) => {
            update_param(draft, name, |param| param.default = optional_string(value))?;
        }
        TemplatePropertyFieldKind::ParamPlaceholder(name) => {
            update_param(draft, name, |param| {
                param.placeholder = optional_string(value)
            })?;
        }
        TemplatePropertyFieldKind::ParamHelp(name) => {
            update_param(draft, name, |param| param.help = optional_string(value))?;
        }
        TemplatePropertyFieldKind::ParamSecret(name) => {
            let secret = parse_bool(value)?;
            update_param(draft, name, |param| param.secret = secret)?;
        }
        TemplatePropertyFieldKind::ParamChoices(name) => {
            let choices = parse_choices_value(value)?;
            update_param(draft, name, |param| param.choices = choices)?;
        }
    }
    Ok(())
}

fn update_param(
    draft: &mut ConfigDraft,
    name: &str,
    update: impl FnOnce(&mut Param),
) -> Result<(), String> {
    let mut params = parse_params_spec(&draft.params)?;
    update(ensure_param(&mut params, name));
    draft.params = params_spec(&params);
    Ok(())
}

fn ensure_param<'a>(params: &'a mut Vec<Param>, name: &str) -> &'a mut Param {
    if let Some(idx) = params.iter().position(|param| param.name == name) {
        return &mut params[idx];
    }
    params.push(Param {
        name: name.to_string(),
        label: None,
        default: None,
        placeholder: None,
        help: None,
        secret: false,
        choices: None,
    });
    params.last_mut().unwrap()
}

fn ensure_option<'a>(options: &'a mut Vec<OptionDef>, id: &str) -> &'a mut OptionDef {
    if let Some(idx) = options.iter().position(|option| option.id == id) {
        return &mut options[idx];
    }
    options.push(OptionDef {
        id: id.to_string(),
        label: None,
        default_enabled: false,
    });
    options.last_mut().unwrap()
}

fn parse_choices_value(value: &str) -> Result<Option<Vec<String>>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with('[') {
        return toml::from_str::<ChoicesSpec>(&format!("choices = {value}"))
            .map(|wrapper| Some(wrapper.choices))
            .map_err(|error| error.to_string());
    }
    Ok(Some(
        value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
    ))
}

fn template_part_display(part: &TemplatePart) -> String {
    match &part.kind {
        TemplatePartKind::Required => part.token.clone(),
        TemplatePartKind::Optional { id } => format!("{id} {}", part.token),
    }
}

fn config_edit_target(source: Option<Source>, local_path: Option<PathBuf>) -> ConfigEditTarget {
    match (source, local_path) {
        (Some(Source::Local), Some(path)) => ConfigEditTarget::LocalProject(path),
        _ => ConfigEditTarget::GlobalEditor,
    }
}

fn focus_state_id(focus: Focus) -> &'static str {
    match focus {
        Focus::Categories => "categories",
        Focus::Commands => "commands",
        Focus::Form => "form",
    }
}

fn focus_from_state_id(value: &str) -> Option<Focus> {
    match value {
        "categories" => Some(Focus::Categories),
        "commands" => Some(Focus::Commands),
        "form" => Some(Focus::Form),
        _ => None,
    }
}

fn adjust_setting_value(settings: &mut Settings, idx: usize, forward: bool) {
    match idx {
        0 => {
            settings.language = match settings.language {
                Language::ZhCn => Language::En,
                Language::En => Language::ZhCn,
            };
        }
        1 => settings.remember_last_selection = !settings.remember_last_selection,
        2 => settings.remember_last_input = !settings.remember_last_input,
        3 if forward => {
            settings.input_record_limit = settings.input_record_limit.saturating_add(1).min(999);
        }
        3 => settings.input_record_limit = settings.input_record_limit.saturating_sub(1).max(1),
        _ => {}
    }
}

fn byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .map(|(idx, _)| idx)
        .nth(char_index)
        .unwrap_or(value.len())
}

fn clamp_text_cursor(cursor: &mut usize, value: &str) {
    *cursor = (*cursor).min(value.chars().count());
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

fn wrapped_text_line_count(text: &str, width: usize) -> usize {
    if text.is_empty() {
        return 1;
    }
    text.split('\n')
        .map(|line| {
            let chars = line.chars().count().max(1);
            chars.div_ceil(width)
        })
        .sum()
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
    use crate::i18n::Language;
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
        app.values.insert("path".to_string(), String::new());
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

    #[test]
    fn empty_config_preview_uses_configured_language() {
        let app = App::new(Config {
            settings: Settings {
                language: Language::En,
                ..Settings::default()
            },
            ..Config::default()
        });

        let preview = app.preview_text();

        assert!(preview.contains("No commands available"));
        assert!(preview.contains("~/.config/cmdp/"));
    }

    #[test]
    fn preview_scroll_max_counts_wrapped_command_lines() {
        let mut config = test_config();
        config.commands.get_mut("find_large").unwrap().template =
            "echo 1234567890 1234567890 1234567890".to_string();
        let mut app = App::new(config);
        app.category_idx = 0;
        app.command_idx = 0;

        assert_eq!(app.preview_scroll_max(10, 1), 3);

        app.scroll_preview(true, 10, 1);
        assert_eq!(app.preview_scroll, 1);
        app.scroll_preview(false, 10, 1);
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn help_window_can_be_toggled() {
        let mut app = App::new(test_config());

        app.toggle_help();
        assert!(app.show_help);

        app.close_help();
        assert!(!app.show_help);
    }

    #[test]
    fn applies_last_selection_by_stable_command_id() {
        let mut app = App::new(test_config());

        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("cargo_check".to_string()),
            focus: None,
            input_records: Vec::new(),
        });

        assert_eq!(app.current_category_id().map(String::as_str), Some("dev"));
        assert_eq!(
            app.current_command().map(|(id, _)| id.as_str()),
            Some("cargo_check")
        );
    }

    #[test]
    fn applies_last_focus_from_state() {
        let mut app = App::new(test_config());

        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("find_large".to_string()),
            focus: Some("form".to_string()),
            input_records: Vec::new(),
        });

        assert_eq!(app.focus, Focus::Form);
    }

    #[test]
    fn applies_last_input_for_current_command_only() {
        let mut app = App::new(test_config());
        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("find_large".to_string()),
            focus: None,
            input_records: Vec::new(),
        });
        app.values.insert("path".to_string(), ".".to_string());

        app.apply_current_input(&AppState {
            category_id: None,
            command_id: None,
            focus: None,
            input_records: vec![InputRecord {
                command_id: "find_large".to_string(),
                values: [("path".to_string(), "./src".to_string())].into(),
                enabled: vec!["unused".to_string()],
            }],
        });

        assert_eq!(app.values.get("path").map(String::as_str), Some("./src"));
        assert!(app.enabled.is_empty());
    }

    #[test]
    fn reset_current_form_returns_to_config_defaults() {
        let mut app = App::new(test_config());
        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("find_large".to_string()),
            focus: None,
            input_records: Vec::new(),
        });
        app.values
            .insert("path".to_string(), "./changed".to_string());
        app.enabled.clear();
        app.error = Some("stale error".to_string());

        app.reset_current_form_to_defaults();

        assert_eq!(app.values.get("path").map(String::as_str), Some("."));
        assert!(app.enabled.contains("hidden"));
        assert_eq!(app.form_idx, 0);
        assert!(app.error.is_none());
    }

    #[test]
    fn file_picker_select_updates_current_param() {
        let mut app = App::new(test_config());
        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("find_large".to_string()),
            focus: None,
            input_records: Vec::new(),
        });
        app.focus = Focus::Form;
        app.file_picker = Some(FilePicker {
            param_name: "path".to_string(),
            dir: PathBuf::from("."),
            entries: vec![FilePickerEntry {
                name: "Cargo.toml".to_string(),
                path: PathBuf::from("Cargo.toml"),
                is_dir: false,
            }],
            selected: 0,
            error: None,
        });

        app.file_picker_select();

        assert_eq!(
            app.values.get("path").map(String::as_str),
            Some("Cargo.toml")
        );
        assert!(app.file_picker.is_none());
    }

    #[test]
    fn file_picker_selects_current_directory_entry() {
        let mut app = App::new(test_config());
        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("find_large".to_string()),
            focus: None,
            input_records: Vec::new(),
        });
        let dir = std::env::current_dir().unwrap();
        app.file_picker = Some(FilePicker {
            param_name: "path".to_string(),
            dir: dir.clone(),
            entries: vec![FilePickerEntry {
                name: ".".to_string(),
                path: dir,
                is_dir: true,
            }],
            selected: 0,
            error: None,
        });

        app.file_picker_select();

        assert_eq!(app.values.get("path").map(String::as_str), Some("."));
        assert!(app.file_picker.is_none());
    }

    #[test]
    fn file_picker_only_opens_for_text_params() {
        let mut app = App::new(test_config());
        app.focus = Focus::Form;
        app.form_idx = 1;

        app.open_file_picker();

        assert!(app.file_picker.is_none());
        assert!(
            app.error
                .as_deref()
                .is_some_and(|error| error.contains("不能打开文件选择"))
        );
    }

    #[test]
    fn file_picker_opens_for_selected_text_param_outside_form_focus() {
        let mut app = App::new(test_config());
        app.apply_selection_state(&AppState {
            category_id: Some("file".to_string()),
            command_id: Some("find_large".to_string()),
            focus: None,
            input_records: Vec::new(),
        });
        app.focus = Focus::Commands;
        app.form_idx = 0;

        app.open_file_picker();

        let picker = app.file_picker.as_ref().unwrap();
        assert_eq!(picker.param_name, "path");
        assert_eq!(app.focus, Focus::Form);
    }

    #[test]
    fn file_entries_sort_directories_first() {
        let dir = temp_app_dir();
        fs::create_dir_all(dir.join("z_dir")).unwrap();
        fs::write(dir.join("a_file"), "").unwrap();

        let entries = read_file_entries(&dir, Language::ZhCn.texts()).unwrap();

        assert_eq!(entries[0].name, ".");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "z_dir");
        assert!(entries[1].is_dir);
        assert_eq!(entries[2].name, "a_file");
        assert!(!entries[2].is_dir);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clamps_input_records_to_configured_limit() {
        let mut app = App::new(test_config());
        app.config.settings.input_record_limit = 1;
        let mut state = AppState {
            category_id: None,
            command_id: None,
            focus: None,
            input_records: vec![
                InputRecord {
                    command_id: "first".to_string(),
                    values: Default::default(),
                    enabled: Vec::new(),
                },
                InputRecord {
                    command_id: "second".to_string(),
                    values: Default::default(),
                    enabled: Vec::new(),
                },
            ],
        };

        app.clamp_input_records(&mut state);

        assert_eq!(state.input_records.len(), 1);
        assert_eq!(state.input_records[0].command_id, "first");
    }

    #[test]
    fn setting_adjustment_updates_selected_setting() {
        let mut settings = Settings::default();

        adjust_setting_value(&mut settings, 0, true);
        assert_eq!(settings.language, Language::En);

        adjust_setting_value(&mut settings, 1, true);
        assert!(settings.remember_last_selection);

        adjust_setting_value(&mut settings, 2, true);
        assert!(settings.remember_last_input);

        adjust_setting_value(&mut settings, 3, true);
        assert_eq!(settings.input_record_limit, DEFAULT_INPUT_RECORD_LIMIT + 1);

        adjust_setting_value(&mut settings, 3, false);
        assert_eq!(settings.input_record_limit, DEFAULT_INPUT_RECORD_LIMIT);
    }

    #[test]
    fn params_spec_parses_all_param_fields() {
        let params = parse_params_spec(
            r#"[{ name = "path", label = "路径", default = ".", placeholder = "./src", help = "选择目录", secret = true, choices = [".", "./src"] }]"#,
        )
        .unwrap();

        assert_eq!(params[0].name, "path");
        assert_eq!(params[0].label.as_deref(), Some("路径"));
        assert_eq!(params[0].default.as_deref(), Some("."));
        assert_eq!(params[0].placeholder.as_deref(), Some("./src"));
        assert_eq!(params[0].help.as_deref(), Some("选择目录"));
        assert!(params[0].secret);
        assert_eq!(
            params[0].choices.as_ref().unwrap(),
            &vec![".".to_string(), "./src".to_string()]
        );

        let spec = params_spec(&params);
        assert!(spec.contains(r#"name = "path""#));
        assert!(spec.contains(r#"choices = [".", "./src"]"#));
    }

    #[test]
    fn params_spec_keeps_legacy_name_label_shorthand() {
        let params = parse_params_spec("path:路径, pattern").unwrap();

        assert_eq!(params[0].name, "path");
        assert_eq!(params[0].label.as_deref(), Some("路径"));
        assert_eq!(params[1].name, "pattern");
        assert!(params[1].label.is_none());
    }

    #[test]
    fn options_spec_parses_option_fields() {
        let options = parse_options_spec(
            r#"[{ id = "hidden", label = "显示隐藏文件", default_enabled = true }]"#,
        )
        .unwrap();

        assert_eq!(options[0].id, "hidden");
        assert_eq!(options[0].label.as_deref(), Some("显示隐藏文件"));
        assert!(options[0].default_enabled);

        let spec = options_spec(&options);
        assert!(spec.contains(r#"id = "hidden""#));
        assert!(spec.contains("default_enabled = true"));
    }

    #[test]
    fn config_editor_field_previews_use_display_labels() {
        assert_eq!(
            params_label_preview(r#"[{ name = "path", label = "路径" }, { name = "pattern" }]"#)
                .unwrap(),
            "路径, pattern"
        );
        assert_eq!(
            options_label_preview(r#"[{ id = "glob", label = "Glob 过滤" }, { id = "hidden" }]"#)
                .unwrap(),
            "Glob 过滤, hidden"
        );
    }

    #[test]
    fn template_parts_extract_required_and_optional_segments() {
        let parts =
            template_parts("find <<{{path}}>> [[name:-name {{name}}]] [[hidden:-hidden]]").unwrap();

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].kind, TemplatePartKind::Required);
        assert_eq!(parts[0].params, vec!["path"]);
        assert_eq!(
            parts[1].kind,
            TemplatePartKind::Optional {
                id: "name".to_string()
            }
        );
        assert_eq!(parts[1].params, vec!["name"]);
        assert_eq!(
            parts[2].kind,
            TemplatePartKind::Optional {
                id: "hidden".to_string()
            }
        );
        assert!(parts[2].params.is_empty());
    }

    #[test]
    fn template_property_edit_updates_param_attributes() {
        let mut draft = config_draft("find <<{{path}}>>");

        set_template_property(
            &mut draft,
            &TemplatePropertyFieldKind::ParamLabel("path".to_string()),
            "路径",
        )
        .unwrap();
        set_template_property(
            &mut draft,
            &TemplatePropertyFieldKind::ParamPlaceholder("path".to_string()),
            "./src",
        )
        .unwrap();

        let params = parse_params_spec(&draft.params).unwrap();
        assert_eq!(params[0].name, "path");
        assert_eq!(params[0].label.as_deref(), Some("路径"));
        assert_eq!(params[0].placeholder.as_deref(), Some("./src"));
    }

    #[test]
    fn template_property_edit_updates_option_and_optional_param_attributes() {
        let mut draft = config_draft("find [[glob:-name {{glob}}]]");

        set_template_property(
            &mut draft,
            &TemplatePropertyFieldKind::OptionLabel("glob".to_string()),
            "按名称过滤",
        )
        .unwrap();
        set_template_property(
            &mut draft,
            &TemplatePropertyFieldKind::OptionDefaultEnabled("glob".to_string()),
            "true",
        )
        .unwrap();
        set_template_property(
            &mut draft,
            &TemplatePropertyFieldKind::ParamPlaceholder("glob".to_string()),
            "*.rs",
        )
        .unwrap();

        let options = parse_options_spec(&draft.options).unwrap();
        assert_eq!(options[0].id, "glob");
        assert_eq!(options[0].label.as_deref(), Some("按名称过滤"));
        assert!(options[0].default_enabled);
        let params = parse_params_spec(&draft.params).unwrap();
        assert_eq!(params[0].placeholder.as_deref(), Some("*.rs"));
    }

    #[test]
    fn invalid_template_property_keeps_editor_open() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        app.open_config_template_property_editor(0);
        app.select_config_template_property(4, false);
        app.begin_config_template_property_edit();
        {
            let property_editor = app
                .config_editor
                .as_mut()
                .unwrap()
                .template_property_editor
                .as_mut()
                .unwrap();
            property_editor.edit_buffer = "not-bool".to_string();
        }

        app.commit_config_template_property_edit();

        let property_editor = app
            .config_editor
            .as_ref()
            .unwrap()
            .template_property_editor
            .as_ref()
            .unwrap();
        assert!(property_editor.editing);
        assert!(
            app.error
                .as_deref()
                .is_some_and(|error| error.contains("invalid bool"))
        );
    }

    #[test]
    fn template_part_labels_use_param_and_option_labels() {
        let mut draft = config_draft("find <<{{path}}>> [[glob:-name {{glob}}]]");
        draft.params =
            r#"[{ name = "path", label = "路径" }, { name = "glob", label = "文件名" }]"#
                .to_string();
        draft.options = r#"[{ id = "glob", label = "按名称过滤" }]"#.to_string();
        let parts = template_parts(&draft.template).unwrap();

        assert_eq!(template_part_labels(&draft, &parts[0]).unwrap(), "路径");
        assert_eq!(
            template_part_labels(&draft, &parts[1]).unwrap(),
            "按名称过滤  文件名"
        );
    }

    #[test]
    fn local_config_commands_are_saved_back_to_local_config() {
        let target = config_edit_target(
            Some(Source::Local),
            Some(PathBuf::from("/work/project/.cmdp.toml")),
        );

        assert_eq!(
            target,
            ConfigEditTarget::LocalProject(PathBuf::from("/work/project/.cmdp.toml"))
        );
    }

    #[test]
    fn global_config_commands_use_global_editor_file() {
        let target = config_edit_target(
            Some(Source::Global),
            Some(PathBuf::from("/work/project/.cmdp.toml")),
        );

        assert_eq!(target, ConfigEditTarget::GlobalEditor);
    }

    fn config_draft(template: &str) -> ConfigDraft {
        ConfigDraft {
            command_id: "find_file".to_string(),
            category_id: "file".to_string(),
            category_alias: "文件".to_string(),
            title: "查找文件".to_string(),
            description: String::new(),
            danger: "false".to_string(),
            template: template.to_string(),
            params: "[]".to_string(),
            options: "[]".to_string(),
        }
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
        let mut find_large = command(
            "file",
            "查找大文件",
            "find <<{{path}}>> [[hidden:-hidden]] [[size:-size +{{size}}]]",
            Source::Global,
        );
        find_large.params.push(Param {
            name: "path".to_string(),
            label: Some("路径".to_string()),
            default: Some(".".to_string()),
            placeholder: None,
            help: None,
            secret: false,
            choices: None,
        });
        find_large.options.push(OptionDef {
            id: "hidden".to_string(),
            label: Some("包含隐藏文件".to_string()),
            default_enabled: true,
        });
        find_large.options.push(OptionDef {
            id: "size".to_string(),
            label: Some("按大小过滤".to_string()),
            default_enabled: false,
        });
        commands.insert("find_large".to_string(), find_large);
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
            settings: Settings::default(),
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

    fn temp_app_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cmdp-app-test-{}-{nonce}", std::process::id()))
    }
}
