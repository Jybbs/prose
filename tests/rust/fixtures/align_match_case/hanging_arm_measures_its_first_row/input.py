def dispatch(command, args):
    match command:
        case "build":
            target = args[0]
        case "install": targets = list(
            resolve(args, with_a_long_argument_name, and_another_long_argument_name)
        )
