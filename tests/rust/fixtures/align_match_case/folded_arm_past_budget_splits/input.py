def pending(args):
    match args:
        case ("gh", group, verb, *_) : return verb in {"close", "merge"}
        case ("mise", *_)            : return pending_labels + pending_rulesets + pending_flows
    return 0
