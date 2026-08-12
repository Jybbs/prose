w = Process(ctx, target=worker,
            args=(
                inqueue, outqueue, initializer,
                initargs, maxtasksperchild, wrap_exception
            ))
