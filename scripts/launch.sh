#!/usr/bin/env bash
set -euo pipefail
project_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$project_directory/scripts/start_offline.py" --app "$project_directory/target/debug/live-translate-rs" "$@"
