x = (r'pipe-%d-%d-%s' %
     (os.getpid(), next(_mmap_counter), os.urandom(8).hex()))
