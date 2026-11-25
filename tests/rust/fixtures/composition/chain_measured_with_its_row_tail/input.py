def mode():
    if int(_os.uname().release.split(".")[0]) < 8:
        return RTLD_GLOBAL
