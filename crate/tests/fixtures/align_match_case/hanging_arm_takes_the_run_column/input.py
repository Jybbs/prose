def dispatch(command, args):
    match command:
        case "build":
            target = args[0]
        case "uninstall":
            target = None
        case "install": targets = list(
            resolve(args)
        )
        case "format":
            target = "."
        case "lint-everything":
            target = "all"
