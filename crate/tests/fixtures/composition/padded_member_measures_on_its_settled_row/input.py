def candidates():
    dirlist = []
    if _os.name == "nt":
        dirlist.extend([ _os.path.expanduser(r"~\AppData\Local\Temp"),
                         _os.path.expandvars(r"%SYSTEMROOT%\Temp") ])
    return dirlist
