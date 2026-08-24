upload_report() {
  local flag="$1" file="$2" parent="${PARENT:-}"

  if [[ "$parent" =~ ^0+$ ]]; then
    parent=""
  fi

  codecovcli upload-process \
    --file "$file" \
    --flag "$flag" \
    ${parent:+--parent-sha "$parent"} \
    --sha "$SHA"
}
