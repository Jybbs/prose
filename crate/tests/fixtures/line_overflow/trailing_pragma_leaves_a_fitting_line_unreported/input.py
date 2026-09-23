reader = getattr(spec.loader, "get_resource_reader", None)  # type: ignore[union-attr, unused-ignore]
handler = resolve_handler(request.method, request.path)  # noted for later  # noqa: E501, F401
fallback = resolve_handler(request.method, request.path)  # an ordinary comment carrying the row past
