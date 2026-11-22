class bdist_egg(Command):
    def get_ext_outputs(self):
        for base, dirs, files in sorted_walk(self.bdist_dir):
            for filename in dirs:
                paths[os.path.join(base, filename)] = (paths[base] +
                                                       filename + '/')
