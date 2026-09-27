def collect(results, exc):
    results.append({"faultCode": 1, "faultString": "%s:%s" % (type(exc), exc)})
