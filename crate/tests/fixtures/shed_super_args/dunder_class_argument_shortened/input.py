class Node:
    def clone(self):
        return self


class Leaf(Node):
    def clone(self):
        return super(__class__, self).clone()
