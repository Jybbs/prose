def collect(results, exc):
    results.append({"faultCode": 1, "faultString": r"%s:%s" % ( type(exc), exc )})
