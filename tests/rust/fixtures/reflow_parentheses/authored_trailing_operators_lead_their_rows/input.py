def gate(session):
    if (session.is_authenticated and
            session.has_permission and
            session.is_within_window):
        return
