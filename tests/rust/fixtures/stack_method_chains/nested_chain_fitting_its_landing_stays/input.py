posix_path = ((subdir / name).resolve().relative_to(root.resolve().parent()).as_posix())
