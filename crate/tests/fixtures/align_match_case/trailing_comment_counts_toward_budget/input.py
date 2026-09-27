def mutates(args):
    match args:
        case "tag":
            return tag_of(args.release_name, args.channel)  # pins the release channel
        case "other":
            return 0
    return False
