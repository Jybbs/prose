def mutates(args):
    match args:
        case ("gh", group, verb, *_):
            return verb in {"close", "merge"}
        case "hold":  # prose: skip[align-match-case]
            return 0
        case ("mise", *_):
            return runs_task(args, {"gha:labels", "gha:rulesets"})
        case "tag":
            return 1
    return False
