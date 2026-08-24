baseline_root() {
  local tree="${1:-main}"
  [[ -d "$tree" ]] || tree="$MISE_PROJECT_ROOT/../$tree"
  git -C "$tree" rev-parse --show-toplevel
}

built_binary() {
  cargo build \
    --bin prose \
    --locked \
    --message-format json-render-diagnostics \
    --profile "$1" \
    | jq -r 'select(.executable != null and .target.name == "prose") | .executable'
}
