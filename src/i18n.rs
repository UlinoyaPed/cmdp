use serde::{Deserialize, Deserializer, de};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    ZhCn,
    En,
}

pub struct Texts {
    pub mode_editing: &'static str,
    pub mode_search: &'static str,
    pub mode_normal: &'static str,
    pub search_label: &'static str,
    pub header_shortcuts: &'static str,
    pub preview_title: &'static str,
    pub execute_label: &'static str,
    pub confirm_label: &'static str,
    pub help_title: &'static str,
    pub help_toggle: &'static str,
    pub help_close_popup_or_search: &'static str,
    pub help_quit: &'static str,
    pub help_switch_area: &'static str,
    pub help_move_selection: &'static str,
    pub help_enter: &'static str,
    pub help_space: &'static str,
    pub help_search: &'static str,
    pub help_file_picker: &'static str,
    pub help_reset_defaults: &'static str,
    pub help_reload_config: &'static str,
    pub help_run_current: &'static str,
    pub file_picker_title_prefix: &'static str,
    pub file_picker_help: &'static str,
    pub target_parameter: &'static str,
    pub empty_directory: &'static str,
    pub categories_title: &'static str,
    pub commands_title: &'static str,
    pub no_matching_commands: &'static str,
    pub config_not_loaded: &'static str,
    pub no_commands: &'static str,
    pub no_params_or_options: &'static str,
    pub form_title: &'static str,
    pub input_placeholder: &'static str,
    pub source_global_local: &'static str,
    pub source_global: &'static str,
    pub source_local: &'static str,
    pub source_none: &'static str,
    pub danger_preview: &'static str,
    pub missing_params_prefix: &'static str,
    pub not_text_param_file_picker: &'static str,
    pub empty_config_preview: &'static str,
    pub no_available_command: &'static str,
    pub danger_confirmation: &'static str,
    pub read_last_selection_failed_prefix: &'static str,
    pub save_last_selection_failed_prefix: &'static str,
    pub read_last_input_failed_prefix: &'static str,
    pub save_last_input_failed_prefix: &'static str,
    pub clear_last_input_failed_prefix: &'static str,
    pub read_state_failed_prefix: &'static str,
    pub read_dir_failed_prefix: &'static str,
    pub read_dir_entry_failed_prefix: &'static str,
    pub read_file_type_failed_prefix: &'static str,
}

pub const ZH_CN: Texts = Texts {
    mode_editing: "编辑参数",
    mode_search: "搜索命令",
    mode_normal: "普通",
    search_label: "/ 搜索",
    header_shortcuts: " Tab/←→切换  f文件  F1/?帮助  Ctrl+y执行  q退出",
    preview_title: "预览  Ctrl+y 执行",
    execute_label: "执行",
    confirm_label: "确认",
    help_title: " 快捷键 ",
    help_toggle: " 打开或关闭此窗口",
    help_close_popup_or_search: " 关闭弹窗 / 退出搜索",
    help_quit: " 退出，不执行命令",
    help_switch_area: " 切换区域",
    help_move_selection: " 移动选择",
    help_enter: " 进入区域或编辑参数",
    help_space: " 切换选项或 choices 参数",
    help_search: " 搜索命令",
    help_file_picker: " 为当前输入参数选择文件",
    help_reset_defaults: " 当前命令回到配置默认值",
    help_reload_config: " 重新加载配置",
    help_run_current: " 执行当前命令",
    file_picker_title_prefix: " 文件选择  ",
    file_picker_help: " Enter进入/选择  Space选择高亮项  ./当前目录  ←/Backspace上级  Esc/f关闭 ",
    target_parameter: "目标参数 ",
    empty_directory: "目录为空",
    categories_title: "分类",
    commands_title: "命令",
    no_matching_commands: "无匹配命令",
    config_not_loaded: "未加载配置",
    no_commands: "无命令",
    no_params_or_options: "无参数或可选项",
    form_title: "参数 / 选项",
    input_placeholder: "输入...",
    source_global_local: "全局+本地",
    source_global: "全局",
    source_local: "本地",
    source_none: "无",
    danger_preview: "⚠ 危险命令：配置标记 danger = true",
    missing_params_prefix: "缺失参数：",
    not_text_param_file_picker: "当前项不是可输入参数，不能打开文件选择",
    empty_config_preview: "没有可用命令\n请在 ~/.config/cmdp/ 添加 .toml 配置，或在当前项目创建 .cmdp.toml",
    no_available_command: "没有可用命令",
    danger_confirmation: "危险命令：再次 Ctrl+y 或点击执行确认",
    read_last_selection_failed_prefix: "读取上次选择失败：",
    save_last_selection_failed_prefix: "保存上次选择失败：",
    read_last_input_failed_prefix: "读取上次输入失败：",
    save_last_input_failed_prefix: "保存上次输入失败：",
    clear_last_input_failed_prefix: "清除上次输入失败：",
    read_state_failed_prefix: "读取状态失败：",
    read_dir_failed_prefix: "读取目录失败：",
    read_dir_entry_failed_prefix: "读取目录项失败：",
    read_file_type_failed_prefix: "读取文件类型失败：",
};

pub const EN: Texts = Texts {
    mode_editing: "Editing",
    mode_search: "Search",
    mode_normal: "Normal",
    search_label: "/ Search",
    header_shortcuts: " Tab/←→ Focus  f File  F1/? Help  Ctrl+y Run  q Quit",
    preview_title: "Preview  Ctrl+y Run",
    execute_label: "Run",
    confirm_label: "Confirm",
    help_title: " Shortcuts ",
    help_toggle: " open or close this window",
    help_close_popup_or_search: " close popup / exit search",
    help_quit: " quit without running",
    help_switch_area: " switch area",
    help_move_selection: " move selection",
    help_enter: " enter area or edit parameter",
    help_space: " toggle option or choices parameter",
    help_search: " search commands",
    help_file_picker: " pick a file for the current parameter",
    help_reset_defaults: " reset current command to config defaults",
    help_reload_config: " reload configuration",
    help_run_current: " run current command",
    file_picker_title_prefix: " File picker  ",
    file_picker_help: " Enter open/select  Space select item  ./current dir  ←/Backspace parent  Esc/f close ",
    target_parameter: "Target ",
    empty_directory: "Directory is empty",
    categories_title: "Categories",
    commands_title: "Commands",
    no_matching_commands: "No matching commands",
    config_not_loaded: "No config loaded",
    no_commands: "No commands",
    no_params_or_options: "No parameters or options",
    form_title: "Parameters / Options",
    input_placeholder: "Type...",
    source_global_local: "global+local",
    source_global: "global",
    source_local: "local",
    source_none: "none",
    danger_preview: "⚠ Dangerous command: config sets danger = true",
    missing_params_prefix: "Missing parameters: ",
    not_text_param_file_picker: "Current item is not a text parameter, cannot open file picker",
    empty_config_preview: "No commands available\nAdd .toml files under ~/.config/cmdp/ or create .cmdp.toml in the current project",
    no_available_command: "No commands available",
    danger_confirmation: "Dangerous command: press Ctrl+y again or click Run to confirm",
    read_last_selection_failed_prefix: "Failed to read last selection: ",
    save_last_selection_failed_prefix: "Failed to save last selection: ",
    read_last_input_failed_prefix: "Failed to read last input: ",
    save_last_input_failed_prefix: "Failed to save last input: ",
    clear_last_input_failed_prefix: "Failed to clear last input: ",
    read_state_failed_prefix: "Failed to read state: ",
    read_dir_failed_prefix: "Failed to read directory: ",
    read_dir_entry_failed_prefix: "Failed to read directory entry: ",
    read_file_type_failed_prefix: "Failed to read file type: ",
};

impl Language {
    pub fn from_code(code: &str) -> Option<Self> {
        let normalized = code
            .trim()
            .split('.')
            .next()
            .unwrap_or(code)
            .to_ascii_lowercase()
            .replace('_', "-");
        match normalized.as_str() {
            "zh" | "zh-cn" | "zh-hans" | "cn" | "chinese" => Some(Self::ZhCn),
            "en" | "en-us" | "en-gb" | "english" => Some(Self::En),
            _ => None,
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self {
            Self::ZhCn => &ZH_CN,
            Self::En => &EN,
        }
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_code(&value)
            .ok_or_else(|| de::Error::custom(format!("unsupported language '{value}'")))
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn language_codes_accept_common_aliases() {
        assert_eq!(Language::from_code("zh-CN"), Some(Language::ZhCn));
        assert_eq!(Language::from_code("zh_CN.UTF-8"), Some(Language::ZhCn));
        assert_eq!(Language::from_code("en-US"), Some(Language::En));
        assert_eq!(Language::from_code("english"), Some(Language::En));
        assert_eq!(Language::from_code("fr"), None);
    }
}
