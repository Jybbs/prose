PROOF_TARGETS=(corpus settle)

split_proof() {
  local -a selected
  local target
  for target in "${PROOF_TARGETS[@]}"; do
    selected+=(--test "$target")
  done

  "$@" --locked --package prose "${selected[@]}"
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
  local target
  cargo metadata --format-version 1 --no-deps \
    | jq -r '.packages[].targets[] | select(.kind == ["test"]) | .name' \
    | while IFS= read -r target; do
        [[ " ${PROOF_TARGETS[*]} " == *" $target "* ]] \
          || printf -- '--test %s\n' "$target"
      done
}
