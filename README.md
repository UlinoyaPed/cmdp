# cmdp

`cmdp` is a ratatui-based TUI command template picker. It loads command templates from TOML configuration, lets users choose categories and commands, fill parameters, toggle optional fragments, preview the rendered command, and then print only the selected command back to the original terminal.

## CI

The GitHub Actions workflow in `.github/workflows/ci.yml` runs:

- `cargo fmt --all -- --check`
- `cargo check --all-targets`
- `cargo clippy --all-targets`

Clippy is intentionally advisory/weak in CI: it does not use `-D warnings`, so new warnings are visible in logs without failing an otherwise buildable change.
