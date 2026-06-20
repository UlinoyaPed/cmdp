#!/usr/bin/env bash
set -euo pipefail

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_cmd cargo
require_cmd cargo-deb
require_cmd cargo-generate-rpm
require_cmd python3

python3 - <<'PY'
from pathlib import Path

try:
    import yaml
except ImportError as exc:
    raise SystemExit("missing Python module: yaml") from exc

for path in (Path(".github/workflows/ci.yml"), Path(".github/workflows/release.yml")):
    with path.open(encoding="utf-8") as handle:
        yaml.safe_load(handle)
    print(f"validated YAML: {path}")
PY

cargo generate-lockfile
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked

tmpdir="${TMPDIR:-/tmp}/cmdp-release-check-$$"
trap 'rm -rf "$tmpdir"' EXIT
mkdir -p "$tmpdir"

cargo deb --no-build --locked --output "$tmpdir"
cargo generate-rpm --output "$tmpdir"

find "$tmpdir" -maxdepth 1 -type f \( -name '*.deb' -o -name '*.rpm' \) -print
