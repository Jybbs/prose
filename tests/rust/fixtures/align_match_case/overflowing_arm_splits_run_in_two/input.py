def classify(event):
    match event.kind:
        case "created":
            label = "new"
        case "archived":
            label = "old"
        case "moved":
            label = moved(event.source_path, event.target_path, event.kind)
        case "deleted_forever":
            label = "gone"
        case "restored":
            label = "back"
