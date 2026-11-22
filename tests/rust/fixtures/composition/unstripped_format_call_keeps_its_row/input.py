def collect(results, exc):
    results.append({"faultCode": 1, "faultString": "{}:{}".format( type(exc), exc )})
