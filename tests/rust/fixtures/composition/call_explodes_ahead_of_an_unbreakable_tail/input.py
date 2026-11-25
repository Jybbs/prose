class Graph:
    def bfs(self, start):
        queue, visited = deque([(start, 0)]), {start}
        return queue, visited
