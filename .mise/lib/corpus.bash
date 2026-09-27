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

lock_task() {
  [[ -z "${PROSE_SWEEP_LOCKED:-}" ]] || return 0
  sweep_locked "$0" "$@"
  exit
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

sweep_locked() {
  local lock=/tmp/prose-sweep.lock
  if [[ -n "${PROSE_SWEEP_LOCKED:-}" ]]; then
    "$@"
  elif command -v lockf > /dev/null; then
    lockf -k -s -t 0 "$lock" true \
      || printf 'waiting for the sweep lock at %s\n' "$lock" >&2
    PROSE_SWEEP_LOCKED=1 lockf -k "$lock" "$@"
  elif command -v flock > /dev/null; then
    flock -n "$lock" true || printf 'waiting for the sweep lock at %s\n' "$lock" >&2
    PROSE_SWEEP_LOCKED=1 flock -o "$lock" "$@"
  else
    echo "neither lockf nor flock is installed to take the sweep lock at $lock" >&2
    return 1
  fi
}

sweep_test() {
  cargo test --locked --no-run --profile probe "$@"
  sweep_locked cargo test --locked --profile probe "$@"
}
