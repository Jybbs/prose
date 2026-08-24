def process_env_var(env_var: str) -> Variable:
    return Variable(env_var)


def _parse_marker_var(tokenizer):
    if tokenizer.check("VARIABLE"):
        return process_env_var(tokenizer.read().text.replace(".", "_"))
    return None
