def finish(self):
    loss = self.novelty.wait()
    return self.consolidate() or loss


def store(table, key_of, row):
    key = key_of(row)
    table[key] = row.refresh()
