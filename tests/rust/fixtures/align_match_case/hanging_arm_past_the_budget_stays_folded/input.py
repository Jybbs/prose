def route(request):
    match request.method:
        case "GET"  : return render_template_with_its_request_context_and_all_of_its_defaults(
            request.path
        )
        case "POST":
            return accept(request)
