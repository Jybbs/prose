positionals = get_positional_actions()
a = [action for action in positionals
     if action.nargs in [PARSER, REMAINDER]]
