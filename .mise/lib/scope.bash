scope() {
  case "${1:-all}" in
    all)  rust && site ;;
    rust) rust ;;
    site) shift && site "$@" ;;
    *)    echo "usage: mise $MISE_TASK_NAME [rust|site]" >&2; exit 2 ;;
  esac
}

split_workspace() {
  "$@" --workspace --exclude prose_wasm --locked
  "$@" --package prose_wasm --locked
}
