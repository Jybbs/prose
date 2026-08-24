split_workspace() {
  "$@" --exclude prose_wasm --locked --workspace
  "$@" --locked --package prose_wasm
}
