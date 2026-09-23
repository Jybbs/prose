def mutates(args):
    match args:
        case ("mise", *_):
            return runs_task(args, {"gha:labels", "gha:rulesets"})
        case ("gh", group, verb, *_):
            return verb in {"close", "merge"}
    return False
