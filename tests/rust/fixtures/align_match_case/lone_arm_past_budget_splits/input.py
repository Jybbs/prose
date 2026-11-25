def tally(event):
    match event.kind:
        case "rebuilt": return source_path_total + target_path_total + flags_total + mode_total
