rows = fetch("%s=%s" % (field.name.strip().lower(), value)).order_by(name).all()
