PYTHON_PATHSPEC=(
  '*.py'
  ':!site/.vitepress/lib/readme/retry.py'
  ':!tests/rust/fixtures'
  ':!tests/site/rules/fixtures'
)

python_prose() {
  local file sources=() subcommand="$1"
  shift
  while IFS= read -r -d '' file; do
    if [[ -f $file ]]; then
      sources+=("$file")
    fi
  done < <(git ls-files -z -- "${PYTHON_PATHSPEC[@]}")

  prose "$subcommand" --no-cache "$@" "${sources[@]}"
}
