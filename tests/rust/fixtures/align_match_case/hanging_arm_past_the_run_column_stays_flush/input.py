def route(request):
    match request.method, request.path:
        case "GET", "/":
            return index()
        case "POST", "/submissions":
            return accept(request)
        case "GET", _             : return render_template_with_its_request_context_and_defaults(
            request.path
        )
