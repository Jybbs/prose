scope() {
  case "${1:-all}" in
    all)  rust && site ;;
    rust) rust ;;
    site) shift && site "$@" ;;
    *)    echo "usage: mise <task> [rust|site]" >&2 && exit 2 ;;
  esac
}
