def kind(event):
    match event.kind:
        case "b":
            result = compute(event.alpha_argument, event.beta_argument, event.x)
        case "an_arm_past_the_shift_cap":
            result = 2
