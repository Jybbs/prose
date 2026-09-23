match event.kind:
    case "tag":
        release = tag_of(event.release_name, event.channel)     # pinned stable
    case "other":
        release = None
