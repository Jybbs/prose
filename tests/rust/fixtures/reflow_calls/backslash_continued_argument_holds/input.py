def pop_source(self, instream, lineno):
    if self.debug:
        print('shlex: popping to %s, line %d' \
              % (instream, lineno))
