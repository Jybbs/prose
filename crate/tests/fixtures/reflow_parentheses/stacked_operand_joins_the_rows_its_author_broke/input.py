def caller_frame(frame, root):
    while (frame and os.path.dirname(frame.f_code.co_filename) ==
           root_of_the_package_dir[0]):
        frame = frame.f_back
    if (frame and os.path.dirname(frame.f_code.co_filename_of_the_calling_module) ==
            the_directory_holding_the_logging_package[0]):
        return frame
    return None
