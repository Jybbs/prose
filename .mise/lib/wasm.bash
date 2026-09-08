wasm_cargo() {
  local subcommand="$1"
  shift
  cargo "$subcommand" \
    --locked \
    --package prose_wasm \
    --profile wasm-release \
    --target wasm32-unknown-unknown \
    "$@"
}
