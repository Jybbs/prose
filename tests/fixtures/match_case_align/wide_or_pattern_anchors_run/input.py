match status:
    case "ok" | "pass":
        result = True
    case "warn":
        result = True
    case "fail" | "error" | "panic":
        result = False
