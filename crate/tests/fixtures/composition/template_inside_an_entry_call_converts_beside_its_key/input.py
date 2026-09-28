def collect(results, exc):
    results.append({"faultString": wrap("%s" % (f(x),))})
