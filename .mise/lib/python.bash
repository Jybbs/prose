PYTHON_PATHSPEC=(
  '*.py'
  ':!crate/tests/fixtures'
  ':!site/.vitepress/tests/rules/fixtures'
)

python_prose() {
  local file sources=() subcommand="$1"
  shift
  while IFS= read -r -d '' file; do
    if [[ -f $file ]]; then
      sources+=("$file")
    fi
  done < <(git ls-files -z -- "${PYTHON_PATHSPEC[@]}")

  if (( ${#sources[@]} == 0 )); then
    echo "git tracks no Python file outside the fixture trees" >&2
    return 1
  fi
  prose "$subcommand" --no-cache "$@" "${sources[@]}"
}
