class Frame:
    """
    handler (Callable): The sink each frame is written to.
    codec (bytes): The wire encoding in force.

    Prose between the runs.

    deadline (float): When the dial is abandoned.
    tag (str): The label carried on every frame.
    """
