def mutates(args):
    match args:
        case ("gh", group, verb, *_):
            return verb in {"close", "merge"}
        case ("mise", *_):
            return runs_task(args, {"gha:labels", "gha:rulesets"})
    return False
