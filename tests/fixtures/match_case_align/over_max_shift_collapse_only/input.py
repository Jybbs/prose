match status:
    case "ok":
        result = True
    case "warn":
        result = True
    case "fail" | "error" | "panic" | "abort" | "halt":
        result = False
