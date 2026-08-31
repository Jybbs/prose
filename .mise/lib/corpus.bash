corpus_root() {
  local root
  root="${1:-$(python -c 'import sysconfig; print(sysconfig.get_paths()["stdlib"])')}" \
    || return
  (cd "$root" && pwd)
}

mutate_corpus() {
  printf '\nMutating the corpus under a %ss budget\n' "$3"
  "$MISE_PROJECT_ROOT/.mise/lib/mutate.py" "$@"
}

scratch_dir() {
  mktemp -d "${TMPDIR:-/tmp}/prose-$1.XXXXXX"
}

settle_corpus() {
  local corpus harness="$2"
  corpus=$(corpus_root "$1") || return
  if [[ -n "${PROSE_SETTLE_BIN:-}" ]]; then
    PROSE_SETTLE_CORPUS="$corpus" "$PROSE_SETTLE_BIN/$harness"
  else
    PROSE_SETTLE_CORPUS="$corpus" cargo test --locked --profile probe --test "$harness"
  fi
}
