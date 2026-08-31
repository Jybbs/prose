corpus_root() {
  local root
  root="${1:-$(python -c 'import sysconfig; print(sysconfig.get_paths()["stdlib"])')}" \
    || return
  (cd "$root" && pwd)
}

mutate_corpus() {
  printf '\nMutating the corpus under a %ss budget\n' "$3"
  cargo run --quiet --release --locked -p prose_sweeps --bin mutate -- "$@"
}

scratch_dir() {
  mktemp -d "${TMPDIR:-/tmp}/prose-$1.XXXXXX"
}

settle_corpus() {
  local corpus
  corpus=$(corpus_root "$1") || return
  PROSE_SETTLE_CORPUS="$corpus" cargo test --locked "${@:2}"
}
