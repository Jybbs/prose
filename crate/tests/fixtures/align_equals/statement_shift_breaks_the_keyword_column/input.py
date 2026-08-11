labels_map = make_labels_map(x)
label_width = 4 + len(str(len(labels_map)))
formatter = Formatter(file=file,
                      offset_width=width,
                      label_width=label_width,
                      show_caches=show_caches)

renderer = Renderer(antialias=antialias,
                    color_depth=depth)
