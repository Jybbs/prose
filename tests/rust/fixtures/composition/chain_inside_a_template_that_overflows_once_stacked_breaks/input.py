rows = query.filter("%s:%s" % (gamma.get(key).strip().lower(), delta)).order_by(name).all()
