import multiprocessing as mp
import multiprocessing.connection


def wait(readers):
    return mp.connection.wait(readers)
