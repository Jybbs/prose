def dispatch(request):
    if (request.is_secure and request.user.is_active and request.body):
        return
