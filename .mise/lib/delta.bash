baseline_binary() {
  (cd "$1" && built_binary "${2:-}")
}

baseline_label() {
  printf '%s (%s)\n' "$1" "$(git -C "$1" describe --always --dirty)"
}

baseline_root() {
  local tree="${1:-main}"
  [[ -d "$tree" ]] || tree="$MISE_PROJECT_ROOT/../$tree"
  git -C "$tree" rev-parse --show-toplevel
}

built_binary() {
  local profile="${1:-${PROSE_DELTA_PROFILE:-probe}}"
  grep -q "^\[profile\.$profile\]" Cargo.toml || {
    echo "$PWD defines no $profile profile, building release instead" >&2
    profile=release
  }
  cargo build \
    --bin prose \
    --locked \
    --message-format json-render-diagnostics \
    --profile "$profile" \
    | jq -r 'select(.executable != null and .target.name == "prose") | .executable'
}

delta_widths() {
  echo "${PROSE_DELTA_WIDTHS:-40 50 60 79 88 100}"
}

fail_width() {
  echo "$1 at width $2, stderr at $3.log" >&2
  exit 1
}

format_tagged() {
  local side="$1" binary="$2" width="$3" status=0
  local record="$stage/.git/$side-$width"
  staged reset --hard -q pristine
  printf 'code-line-length = %s\n' "$width" > "$stage/prose.toml"
  "$binary" format --no-cache --output-format json "$stage" \
    2> "$record.log" \
    | jq --arg stage "$stage/" -c '
        if .kind == "summary" then select(.schema_version == 1)
        else {code, filename: (.filename | ltrimstr($stage))} end' \
    > "$record.ndjson" || status=$?
  (( status <= 4 )) || fail_width "$binary exited $status" "$width" "$record"
  jq -es 'map(select(.kind == "summary")) | length == 1 and .[0].files_visited > 0' \
    "$record.ndjson" > /dev/null \
    || fail_width "$binary visited no file" "$width" "$record"
  staged commit --allow-empty -am "$side at width $width" -q
  staged tag -f "$side-$width" > /dev/null
}

format_widths() {
  local width
  for width in $(delta_widths); do
    printf '%s at width %s\n' "$1" "$width"
    format_tagged "$1" "$2" "$width"
  done
}

stage_corpus() {
  staged init -b delta -q
  staged --work-tree="$1" add -f -- '*.py'
  staged commit -m pristine -q
  staged tag pristine
}

staged() {
  git -C "$stage" \
    -c commit.gpgsign=false \
    -c gc.auto=0 \
    -c tag.gpgsign=false \
    -c user.email=delta@prose.fyi \
    -c user.name=prose-delta \
    "$@"
}
