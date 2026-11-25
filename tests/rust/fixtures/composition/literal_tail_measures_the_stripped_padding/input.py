def finish(non_adjacent, la, lb):
    if la or lb:
        non_adjacent.append( (la, lb, 0) )
