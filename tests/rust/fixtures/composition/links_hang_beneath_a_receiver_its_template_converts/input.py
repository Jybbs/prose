rows = fetch("%s:%s" % (alpha, beta)).order_by(name).first().all()
