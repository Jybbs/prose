split_workspace() {
  "$@" --workspace --exclude prose_wasm --locked
  "$@" --package prose_wasm --locked
}
