changed_paths() {
  git diff --merge-base --name-only --no-renames origin/main -- "$@" || return
  git ls-files --exclude-standard --others -- "$@"
}
