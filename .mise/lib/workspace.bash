PROOF_TARGETS=(corpus settle)

split_proof() {
  "$@" --locked --package prose "${PROOF_TARGETS[@]/#/--test=}"
}

split_suite() {
  local targets
  targets=$(suite_targets) || return

  "$@" --bins --exclude prose_wasm --lib --locked --workspace $targets
  "$@" --locked --package prose_wasm
}

split_workspace() {
  "$@" --exclude prose_wasm --locked --workspace
  "$@" --locked --package prose_wasm
}

suite_targets() {
  cargo metadata --format-version 1 --no-deps \
    | jq --args -r '
        .packages[].targets[]
        | select(.kind == ["test"] and (.name | IN($ARGS.positional[]) | not))
        | "--test=" + .name' \
      "${PROOF_TARGETS[@]}"
}
