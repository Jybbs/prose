class Distribution:
    def files(self, subdir, text):
        paths = (
            (subdir / name)
            .resolve()
            .relative_to(self.locate_file('').resolve(), walk_up=True)
            .as_posix()
            for name in text.splitlines()
        )
