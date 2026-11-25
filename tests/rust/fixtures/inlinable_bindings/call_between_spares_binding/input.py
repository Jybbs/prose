def finish(self):
    loss = self.novelty.wait()
    self.consolidate()
    return loss


def tick(counter, clock):
    before = counter.value
    clock.advance()
    return counter.value != before
