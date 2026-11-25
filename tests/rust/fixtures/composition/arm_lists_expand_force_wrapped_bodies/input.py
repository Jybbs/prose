def dispatch(kind):
    match kind:
        case "alpha":
            return ["primary_label", "secondary_label", "tertiary_label", "quaternary_label", "quinary_label"]
        case "beta":
            return ["beta_first", "beta_second", "beta_third", "beta_fourth", "beta_fifth"]
        case _:
            return []
