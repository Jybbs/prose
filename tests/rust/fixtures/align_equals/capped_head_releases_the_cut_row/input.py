def layout(action):
    help_position = min(self.maxlen, 24)
    width = self.width - 1
    indent = help_position - 2
    header = self.fmt(action)
    header_no_color = decolor(header)
