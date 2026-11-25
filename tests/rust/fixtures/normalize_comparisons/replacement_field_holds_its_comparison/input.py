def render(k, table, x):
    label = f"{x == None}"
    other = f"{not k in table}"
    return label + other
