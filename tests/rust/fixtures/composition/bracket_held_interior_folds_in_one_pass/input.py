class Parser:
    def consume(self, action, args, start_index, arg_count):
        if action.nargs != REMAINDER:
            if (arg_strings_pattern.find('-', start_index,
                                         start_index + arg_count) >= 0):
                args.remove('--')
