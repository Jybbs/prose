class Action:
    def __init__(self, dest, help, metavar):
        if dest:
            sup.__init__(option_strings=[], dest=dest, help=help,
                         metavar=metavar)
