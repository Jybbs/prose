corpus_root() {
  local root
  root="${1:-$(python -c 'import sysconfig; print(sysconfig.get_paths()["stdlib"])')}" \
    || return
  (cd "$root" && pwd)
}

corpus_run() {
  local bin="$1"
  shift
  cargo run --bin "$bin" --locked -p prose_corpus --profile probe --quiet -- "$@"
}

mutate_corpus() {
  printf '\nMutating the corpus under a %ss budget\n' "$3"
  corpus_run mutate "$@"
}

scratch_dir() {
  mktemp -d "${TMPDIR:-/tmp}/prose-$1.XXXXXX"
}

settle_corpus() {
  local corpus
  corpus=$(corpus_root "$1") || return
  PROSE_SETTLE_CORPUS="$corpus" cargo test --locked --profile probe "${@:2}"
}
