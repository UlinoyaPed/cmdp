# Repository Guidelines

## Project Structure & Module Organization

This is a Rust terminal UI application built with `ratatui` and `crossterm`.
Source files live in `src/`, with one module per concern:
`main.rs` starts the terminal and event loop, `app.rs` owns UI state,
`ui.rs` draws widgets, `event.rs` handles keyboard and mouse input,
`config.rs` loads and validates TOML configuration, and `parser.rs` /
`renderer.rs` convert command templates into executable command strings.
Example configuration is stored in `examples/commands.toml`. Unit tests are
embedded in the relevant source modules under `#[cfg(test)]`.

## Build, Test, and Development Commands

- `cargo run`: run the TUI locally.
- `cargo build`: compile a debug build.
- `cargo build --release`: compile an optimized binary.
- `cargo test`: run all unit tests.
- `cargo fmt --check`: verify Rust formatting.
- `cargo clippy --all-targets`: run lint checks for the binary and tests.
- `cargo install --path .`: install the current checkout as `cmdp`.
- `scripts/check-release-local.sh`: validate workflows, checks, release build,
  and local `.deb` / `.rpm` packaging before tagging.

## Coding Style & Naming Conventions

Use standard Rust formatting via `rustfmt`; keep code formatted before
committing. Prefer small modules and focused helper functions that match the
existing file boundaries. Use `snake_case` for functions, variables, module
names, command IDs, category IDs, and TOML option IDs. Keep user-facing UI text
short because it renders in constrained terminal columns.

## Testing Guidelines

Use Rust unit tests in the module that owns the behavior being tested. Name
tests after the behavior, for example
`parameter_editing_supports_cursor_movement_and_unicode`. Add tests for parser,
renderer, config-loading, and state-machine changes because regressions there
can affect command execution. Run `cargo test` before submitting changes.

## Commit & Pull Request Guidelines

Use Conventional Commit-style prefixes for every commit:
`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, or `ci:`.
Write messages as `<type>: <short summary>`, for example
`feat: support multiple config files` or `fix: keep cursor movement unicode-safe`.
Keep commits scoped to one behavior change.

Pull requests should describe the user-visible change, mention any config or
keyboard behavior affected, and list validation commands run. Include terminal
screenshots or short recordings when changing TUI layout, labels, or interaction
flow.

## Configuration Notes

Global user configuration is loaded from all first-level `.toml` files in
`~/.config/cmdp/`, then local `.cmdp.toml` files are discovered upward from the
current directory. Preserve this merge order when changing config behavior.
