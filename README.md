# cmdp

`cmdp` 是一个基于 ratatui 的命令模板选择 TUI。它从配置文件读取命令模板、选择分类和命令、填写参数、开关可选片段、实时预览最终命令。确认后，程序会先退出 TUI、恢复终端状态，然后在原终端中执行最终生成的命令。

执行命令前会完整关闭 raw mode、退出 alternate screen，并恢复光标显示，因此命令输出会直接显示在原终端里。

## 项目结构

- `src/main.rs`: 程序入口、终端初始化、事件循环、恢复终端后触发命令执行
- `src/app.rs`: TUI 状态、焦点、选择项、搜索、参数值、可选项状态
- `src/ui.rs`: ratatui 布局和界面绘制
- `src/event.rs`: crossterm 键盘事件处理
- `src/config.rs`: 全局/本地配置发现、读取、合并、校验
- `src/template.rs`: 配置数据结构
- `src/parser.rs`: 模板语法解析和参数使用分析
- `src/renderer.rs`: 根据参数和可选项渲染最终命令
- `src/preview.rs`: 预览文本、缺失参数提示、危险提示
- `src/output.rs`: 使用继承的 `stdin`、`stdout`、`stderr` 启动 shell 子进程执行最终命令
- `src/error.rs`: 配置、模板、渲染相关错误
- `examples/commands.toml`: 示例全局配置
- `.cmdp.toml`: 示例本地项目配置

## 运行

```sh
cargo run
```

常用按键：

- `Tab` / `Shift+Tab`: 在分类、命令、表单之间切换
- `↑` / `↓` 或 `j` / `k`: 移动当前列表或表单选择
- `/`: 快速搜索命令，搜索会跨分类匹配命令 ID、标题、描述、分类和来源
- `Esc`: 退出搜索输入；搜索已退出时清空搜索
- `Enter`: 进入参数编辑，或确认当前表单项
- `Space`: 切换可选片段，或切换 `choices` 参数值
- `Ctrl+r`: 重新加载配置
- `Ctrl+y`: 确认当前命令，退出 TUI，并在原终端执行最终命令
- `q`: 退出，不执行命令

## 配置文件位置

全局配置默认路径：

```text
~/.config/cmdp/commands.toml
```

本地配置文件名：

```text
.cmdp.toml
```

启动时会先加载全局配置，再从当前目录向上查找 `.cmdp.toml`，直到用户家目录或文件系统根目录。本地配置可以追加分类和命令，也可以用同名命令整体覆盖全局命令。

首次运行时，如果 `~/.config/cmdp` 目录不存在，程序会创建一个最小示例 `commands.toml`。如果之后删除或清空配置文件，程序不会继续显示内置命令。

## 配置文件教程

配置使用 TOML。分类和命令分开定义，参数和可选项直接写在对应命令块内。

最小结构：

```toml
version = 1

[categories.dev]
alias = "开发工具"

[commands.cargo_check]
category = "dev"
title = "Cargo Check"
description = "在当前目录运行 cargo check"
danger = false
template = '''
cargo check [[all_targets:--all-targets]] [[features:--features {{features}}]]
'''

params = [
  { name = "features", label = "Features", placeholder = "serde,cli" },
]

options = [
  { id = "all_targets", label = "检查所有 target", default_enabled = false },
  { id = "features", label = "启用 features", default_enabled = false },
]
```

分类写在 `[categories.<category_id>]` 下：

```toml
[categories.file]
alias = "文件管理"
```

`category_id` 使用英文作为稳定 ID，`alias` 只用于界面显示。分类显示顺序就是配置文件里的定义顺序，不需要 `order` 字段。

命令写在 `[commands.<command_id>]` 下：

```toml
[commands.find_large]
category = "file"
title = "查找大文件"
description = "查找指定目录下超过给定大小的文件"
danger = false
template = '''
find <<{{path}}>> -type f [[size:-size +{{size}}]]
'''
```

常用字段：

- `category`: 命令所属分类 ID
- `title`: 界面显示标题
- `description`: 命令说明，可被快速搜索匹配
- `danger`: 危险命令标记，预览区会明确提示
- `template`: 命令模板
- `params`: 参数定义数组
- `options`: 可选片段开关定义数组

参数定义：

```toml
params = [
  { name = "path", label = "搜索路径", default = ".", placeholder = "." },
  { name = "pattern", label = "文件名匹配", placeholder = "*.log" },
  { name = "mode", label = "模式", choices = ["fast", "full"] },
  { name = "token", label = "Token", secret = true },
]
```

只有 `name` 必填。所有参数都按原始文本处理，不做自动转义、类型转换、路径检查或数字校验。需要引号时请直接写在模板里，例如 `-name "{{pattern}}"`。

可选项定义：

```toml
options = [
  { id = "ignore_case", label = "忽略大小写", default_enabled = false },
  { id = "glob", label = "启用 Glob 过滤", default_enabled = false },
]
```

`options[].id` 必须对应模板里的命名可选片段 ID，例如 `[[glob:--glob "{{glob}}"]]`。可选片段关闭时，其中的参数不会要求填写；开启后才会参与缺失参数校验。

## 模板语法

- `{{name}}`: 用户输入占位符
- `<<...>>`: 必填片段，片段内参数缺失时不能确认执行
- `[[...]]`: 匿名可选片段，程序会生成内部 ID，默认关闭
- `[[id:...]]`: 命名可选片段，可和 `options = [...]` 对应

示例：

```toml
template = '''
rg [[ignore_case:-i]] [[line_number:-n]] [[glob:--glob "{{glob}}"]] <<{{query}}>> <<{{path}}>>
'''
```

这里 `query` 和 `path` 是必填参数；`ignore_case`、`line_number`、`glob` 是可切换的可选片段；只有启用 `glob` 后，`glob` 参数才必须填写。

第一版不支持嵌套片段，例如 `[[...<<...>>...]]` 或 `<<...[[...]]...>>`。

## 验证

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets
```

## 执行示例

确认命令后，TUI 会退出，然后在原终端执行最终命令。例如预览区生成：

```sh
find . -type f -size +1G -printf '%s\t%p\n' | sort -nr | numfmt --field=1 --to=iec
```

按 `Ctrl+y` 后，`cmdp` 会用当前 shell 执行这条命令，并把子进程的 `stdin`、`stdout`、`stderr` 直接继承到当前终端。
