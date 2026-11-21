PROOF_TARGETS=(corpus settle)

cargo_test() {
  cargo test "$@" ${forwarded[@]+"${forwarded[@]}"}
}

split_proof() {
  "$@" \
    --exclude prose_wasm \
    --locked \
    --workspace \
    ${PROSE_PROOF_PROFILE:+--profile "$PROSE_PROOF_PROFILE"} \
    "${PROOF_TARGETS[@]/#/--test=}"
}

split_suite() {
  local targets
  targets=$(suite_targets) || return

  "$@" --bins --exclude prose_wasm --lib --locked --workspace $targets
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

wasm_reach() {
  cargo metadata --format-version 1 --no-deps \
    | jq -r '
        .workspace_root as $root
        | (.packages
            | map({
                key: (.manifest_path | rtrimstr("/Cargo.toml")),
                value: [.dependencies[] | select(.path and .kind != "dev") | .path]
              })
            | from_entries) as $paths
        | def reach: ., ($paths[.][] | reach);
          [.packages[]
            | select(.name == "prose_wasm")
            | .manifest_path
            | rtrimstr("/Cargo.toml")
            | reach]
        | unique[]
        | ltrimstr($root + "/")'
}

workspace_members() {
  cargo metadata --format-version 1 --no-deps \
    | jq -r '
        .workspace_root as $root
        | .packages[].manifest_path
        | ltrimstr($root + "/")
        | rtrimstr("/Cargo.toml")'
}
