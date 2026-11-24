corpus_build() {
  probe_build --bins --profile probe --quiet
}

corpus_root() {
  local root
  root="${1:-$(python -c 'import sysconfig; print(sysconfig.get_paths()["stdlib"])')}" \
    || return
  (cd "$root" && pwd)
}

corpus_run() {
  local binary
  binary=$(probe_binary "$1" probe --quiet) || return
  "$binary" "${@:2}"
}

corpus_target() {
  python -c 'import sys; print(*sys.version_info[:2], sep=".")'
}

mutate_corpus() {
  printf '\nMutating the corpus under a %ss budget\n' "$3"
  corpus_run mutate "$@"
}

probe_binary() {
  probe_build \
    --bin "$1" \
    --message-format json-render-diagnostics \
    --profile "$2" \
    "${@:3}" \
    | jq --arg name "$1" -r 'select(.target.name == $name).executable // empty'
}

probe_build() {
  cargo build --locked -p prose -p prose_corpus "$@"
}

scratch_dir() {
  mktemp -d "${TMPDIR:-/tmp}/prose-$1.XXXXXX"
}

settle_corpus() {
  local corpus targets
  corpus=$(corpus_root "$1") || return
  targets=${PROSE_SETTLE_TARGETS:-$(corpus_target)} || return
  PROSE_SETTLE_CORPUS="$corpus" PROSE_SETTLE_TARGETS="$targets" sweep_test "${@:2}"
}

sweep_test() {
  cargo test --exclude prose_wasm --locked --profile probe --workspace "$@"
}
