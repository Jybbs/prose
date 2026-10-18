def bind(sa, err, total, seq_type_name):
    msg = ('error while attempting '
           'to bind on address %r: %s'
           % (sa, str(err).lower()))
    differing = ('\nFirst %s contains %d additional '
                 'elements past the ones the second sequence holds.\n'
                 % (seq_type_name, total))
    return msg, differing
