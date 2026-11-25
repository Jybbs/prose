def index(rows):
    return dict({key: normalize(value) for key, value in rows})
