class T:
    def test_set(self):
        self.assertEqual(self.tv.set(item),
                         {'B': 123} if self.wantobjects else {'B': '123'})
