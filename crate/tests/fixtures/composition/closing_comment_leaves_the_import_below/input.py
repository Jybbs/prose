import threading  # we want threading
                  # to load first
from subprocess import check_output
import weakref

handle = weakref.ref(threading)
