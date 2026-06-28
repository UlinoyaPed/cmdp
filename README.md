# cmdp

<p align="center"> <a href="https://github.com/UlinoyaPed/cmdp/actions/workflows/ci.yml"><img src="https://github.com/UlinoyaPed/cmdp/actions/workflows/ci.yml/badge.svg?branch=master" alt="CI"></a> <a href="https://github.com/UlinoyaPed/cmdp/actions/workflows/release.yml"><img src="https://github.com/UlinoyaPed/cmdp/actions/workflows/release.yml/badge.svg" alt="Release"></a> <a href="https://github.com/UlinoyaPed/cmdp/releases"><img src="https://img.shields.io/github/v/release/UlinoyaPed/cmdp?sort=semver&display_name=tag&label=release&color=blue" alt="GitHub Release"></a> <a href="https://totapo.eu.org/"><img src="https://img.shields.io/badge/Blog-totapo.eu.org-blue?logo=rss" alt="Blog"></a> <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-brown?logo=rust" alt="Rust"></a> <a href="https://github.com/UlinoyaPed/cmdp/blob/master/LICENSE"><img src="https://img.shields.io/github/license/UlinoyaPed/cmdp" alt="License"></a> </p>

`cmdp` 是一个基于 ratatui 的命令模板选择 TUI。它从配置文件读取命令模板、选择分类和命令、填写参数、开关可选片段、实时预览最终命令。确认后，程序会先退出 TUI、恢复终端状态，然后在原终端中执行最终生成的命令。

执行命令前会完整关闭 raw mode、退出 alternate screen，并恢复光标显示，因此命令输出会直接显示在原终端里。

![用户界面](https://totapo.eu.org/images/e6b3357a-b79b-47cd-b31f-107ba9df51f3.png)

`cmdp` 意为 `Command Palette`

## 项目结构

- `src/main.rs`: 程序入口、终端初始化、事件循环、恢复终端后触发命令执行
- `src/app.rs`: TUI 状态、焦点、选择项、搜索、参数值、可选项状态
- `src/ui.rs`: ratatui 布局和界面绘制
- `src/event.rs`: crossterm 键盘事件处理
- `src/config.rs`: 全局/本地配置发现、读取、合并、校验
- `src/i18n.rs`: 内置界面文案的语言选择和翻译表
- `src/template.rs`: 配置数据结构
- `src/parser.rs`: 模板语法解析和参数使用分析
- `src/renderer.rs`: 根据参数和可选项渲染最终命令
- `src/preview.rs`: 预览文本、缺失参数提示、危险提示
- `src/output.rs`: 使用继承的 `stdin`、`stdout`、`stderr` 启动 shell 子进程执行最终命令
- `src/state.rs`: 读取和写入上次选择、输入记录等本地状态
- `src/error.rs`: 配置、模板、渲染相关错误
- `examples/*.toml`: 按主题拆分的示例全局配置
- `.cmdp.toml`: 当前项目的本地开发配置

## 运行

```sh
cargo run
```

常用按键：

- `Tab` / `Shift+Tab`: 在分类、命令、表单之间切换
- `←` / `→`: 在分类、命令、表单之间左右切换
- `↑` / `↓` 或 `j` / `k`: 移动当前列表或表单选择
- 鼠标左键: 点击分类、命令或表单项；点击表单参数会进入编辑，点击选项会切换；点击标题栏右侧的 `执行` 按钮会确认执行
- 鼠标滚轮: 在分类、命令或表单区域内滚动选择
- `F1`: 在任意状态打开或关闭快捷键提示窗口
- `?`: 在普通模式打开或关闭快捷键提示窗口
- `/`: 快速搜索命令，搜索会跨分类匹配命令 ID、标题、描述、分类和来源
- `f`: 表单焦点停在普通输入参数上时，打开浮动文件选择器；首项 `./` 可选择当前目录
- `Esc`: 退出搜索输入；搜索已退出时清空搜索
- `Enter`: 进入参数编辑，或确认当前表单项
- `Space`: 切换可选片段，或切换 `choices` 参数值
- `Ctrl+d`: 当前命令的参数和可选片段恢复到配置默认值，并清除该命令已记住的输入
- `Ctrl+r`: 重新加载配置
- `Ctrl+y`: 确认当前命令，退出 TUI，并在原终端执行最终命令
- `q`: 退出，不执行命令

危险命令需要二次确认。如果命令配置了 `danger = true`，第一次按 `Ctrl+y` 或点击 `执行` 只会在 TUI 内显示危险确认提示；再次确认同一个渲染后的命令才会退出 TUI 并执行。切换命令、修改参数、切换选项或搜索变化都会取消这次确认。

## 配置文件位置

全局配置目录：

```text
~/.config/cmdp/
```

本地配置文件名：

```text
.cmdp.toml
```

启动时的加载顺序：

1. 确保 `~/.config/cmdp/` 目录存在。
2. 读取该目录第一层所有 `.toml` 文件，按文件名排序后逐个合并。
3. 从当前目录向上查找第一个 `.cmdp.toml`，直到用户家目录或文件系统根目录。
4. 如果找到本地 `.cmdp.toml`，最后加载它。

程序不会递归读取全局配置子目录，也不会在代码里内置或硬编码示例配置。`examples/` 只是仓库里的模板库，不会被自动加载；只有复制到 `~/.config/cmdp/` 后才会成为全局配置。如果全局目录没有 TOML 文件，并且当前路径也没有本地 `.cmdp.toml`，程序就不会显示命令。

当前仓库也带了一个 `.cmdp.toml`，用于覆盖或追加适合本项目的本地命令。你在仓库根目录或子目录启动 `cmdp` 时，会额外看到 `cmdp 开发` 和 `cmdp 发布` 两组命令，例如格式检查、测试、Clippy、本地发布检查、安装当前 checkout、预览 README 和复制示例配置。

记住上次选择默认关闭。需要启用时，创建独立设置文件 `~/.config/cmdp/settings.toml`：

```toml
remember_last_selection = true
remember_last_input = true
input_record_limit = 20
language = "zh-CN"
```

这个文件只放程序设置，不参与命令配置合并；`settings.toml` 也会从全局命令配置列表里排除。开启后，`cmdp` 会把最近选择的分类、命令和输入快照写入 `${XDG_STATE_HOME:-~/.local/state}/cmdp/state.toml`，下次启动或 `Ctrl+r` 重新加载后恢复。在 Unix 系统上，状态文件会以仅当前用户可读写的权限写入。

`language` 控制内置界面语言，默认是 `zh-CN`，也可以设置为 `en`。这个设置会影响标题栏、快捷键帮助、空状态、预览区提示、文件选择器和状态错误等程序自带文案；命令标题、分类别名、参数标签和说明仍由你的 TOML 命令配置决定。

`remember_last_selection` 控制是否恢复上次选中的分类和命令；`remember_last_input` 控制是否按命令 ID 恢复上次输入的普通参数值和可选片段状态；`input_record_limit` 控制最多保留多少条命令输入记录，默认是 `20`。`secret = true` 的参数不会写入状态文件。把对应开关改为 `false` 或删除设置文件即可关闭。

## 示例配置

`examples/` 下的示例配置按使用场景拆分，适合按需复制到全局配置目录：

- `archive.toml`: `tar`、`zip`、`unzip` 压缩解压
- `disc.toml`: xorriso 光盘设备、ISO 制作、刻录和校验
- `file.toml`: `less`、`wc`、`tail` 等文件查看
- `flatpak.toml`: Flatpak 搜索、安装、卸载和权限查看
- `git.toml`: 常用 Git 状态、diff、提交、分支、标签和推送
- `package.dnf.toml`: Fedora/DNF 软件包管理
- `rust.toml`: Cargo run/build/test/fmt/clippy/install
- `search.toml`: `find` 和 `grep` 搜索
- `size.toml`: `du`、`ls`、大文件查找
- `systemd.toml`: systemctl 和 journalctl

安装全部示例：

```sh
mkdir -p ~/.config/cmdp
cp -n examples/*.toml ~/.config/cmdp/
```

只安装部分示例：

```sh
mkdir -p ~/.config/cmdp
cp -n examples/git.toml examples/rust.toml ~/.config/cmdp/
```

本地项目配置写在项目根目录的 `.cmdp.toml`。它适合放项目专属命令，例如这个仓库的本地配置：

```toml
version = 1

[categories.project]
alias = "cmdp 开发"

[commands.cmdp_test]
category = "project"
title = "运行测试"
description = "运行全部测试，或按名称过滤单个测试"
danger = false
template = '''
cargo test [[locked:--locked]] [[test_name:{{test_name}}]] [[nocapture:-- --nocapture]]
'''

params = [
  { name = "test_name", label = "测试过滤", placeholder = "config / renderer" },
]

options = [
  { id = "locked", label = "使用 Cargo.lock", default_enabled = false },
  { id = "test_name", label = "只跑匹配测试", default_enabled = false },
  { id = "nocapture", label = "显示测试输出", default_enabled = false },
]
```

全局和本地配置合并后，如果命令 ID 相同，后加载的本地命令会整体覆盖全局命令。这个规则可以用来给某个项目定制更合适的默认参数、标题或危险标记。

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

`options[].id` 必须对应模板里的命名可选片段 ID，例如 `[[glob:--glob "{{glob}}"]]`。

可选片段关闭时，其中的参数不会要求填写；开启后才会参与缺失参数校验。

## 模板语法

- `{{name}}`: 用户输入占位符
- `<<...>>`: 必填片段，片段内参数缺失时不能确认执行
- `[[...]]`: 匿名可选片段，程序会生成内部 ID，默认关闭
- `[[id:...]]`: 命名可选片段，和 `options = [...]` 对应

示例：

```toml
template = '''
rg [[ignore_case:-i]] [[line_number:-n]] [[glob:--glob "{{glob}}"]] <<{{query}}>> <<{{path}}>>
'''
```

这里 `query` 和 `path` 是必填参数；`ignore_case`、`line_number`、`glob` 是可切换的可选片段；只有启用 `glob` 后，`glob` 参数才必须填写。

不支持嵌套片段，例如 `[[...<<...>>...]]` 或 `<<...[[...]]...>>`。

### 重定向写法

Shell 重定向符 `>`, `>>`, `<`, `2>`, `2>>` 都按普通模板文本处理。推荐把重定向符写在片段外，只把文件路径作为参数：

```toml
template = '''
sort < <<"{{input}}">> > <<"{{output}}">> [[log:2>> "{{log}}"]]
'''

params = [
  { name = "input", label = "输入文件", placeholder = "input.txt" },
  { name = "output", label = "输出文件", placeholder = "output.txt" },
  { name = "log", label = "错误日志", placeholder = "cmd.log" },
]

options = [
  { id = "log", label = "追加 stderr 到日志", default_enabled = false },
]
```

不要把 `>>` 写进 `<<...>>` 必填片段内部；`<<...>>` 是 cmdp 的必填片段语法，内部的第一个 `>>` 会被当作片段结束符。需要追加输出时，使用 `>> <<"{{file}}">>` 或 `[[append:>> "{{file}}"]]` 这类写法。

## 验证

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets
```

发布前可运行本地 release 检查。该脚本会检查 GitHub Actions YAML 语法，执行格式、Clippy、测试和本机 release build，并用本机架构生成 `.deb` / `.rpm` 包：

```sh
scripts/check-release-local.sh
```

## 提交约定

提交信息使用 Conventional Commit 风格：

```text
<type>: <简短说明>
```

常用 `type` 包括 `feat`、`fix`、`docs`、`refactor`、`test`、`chore`、`ci`。例如：

```text
feat: 支持多配置文件
fix: 修复输入光标移动
docs: 更新贡献指南
```

每个提交只包含一个清晰的行为变更；涉及界面、配置或快捷键变化时，请在提交说明或 PR 描述里写明验证命令。

## 发布

推送 `v*` 标签会触发 release workflow。发布前会执行格式检查、Clippy 和测试，然后为以下 Linux 目标编译并上传 `.tar.gz`、`.deb` 和 `.rpm` 包，同时单独上传一份 `examples` 示例配置压缩包：

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `armv7-unknown-linux-gnueabihf`

## 执行示例

确认命令后，TUI 会退出，然后在原终端先打印最终命令，再执行它。例如预览区生成：

```sh
find . -type f -size +1G -printf '%s\t%p\n' | sort -nr | numfmt --field=1 --to=iec
```

按 `Ctrl+y` 后，`cmdp` 会先打印这条命令，然后用当前 shell 执行它，并把子进程的 `stdin`、`stdout`、`stderr` 直接继承到当前终端。
