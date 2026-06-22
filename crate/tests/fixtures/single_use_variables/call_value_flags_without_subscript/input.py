def fetch():
    name, value = lookup()
    return render(name, value)
