reader = getattr(spec.loader, "get_resource_reader", None)  # type: ignore[union-attr, unused-ignore]
handler = resolve_handler(request.method, request.path)  # noted for later  # noqa: E501, F401
fallback = resolve_handler(request.method, request.path)  # an ordinary comment carrying the row past
mapping = collect_entries(request.method, request.path, request.body)  # prose: skip[reflow-calls]  # noqa: E501
summary = compute_totals(first, second)  # an ordinary note long enough to run past the budget here  # prose: ignore[align-equals]
