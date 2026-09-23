def total(payload):
    value = explode_me(alpha_argument, beta_argument, gamma_argument, delta) + payload.get("command_name", "").lstrip("/")
    return value
