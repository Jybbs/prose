corpus_build() {
  cargo build --bins --locked -p prose_corpus --profile probe --quiet
}

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

corpus_target() {
  python -c 'import sys; print(*sys.version_info[:2], sep=".")'
}

mutate_corpus() {
  printf '\nMutating the corpus under a %ss budget\n' "$3"
  corpus_run mutate "$@"
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
  cargo test --locked --profile probe "$@"
}
