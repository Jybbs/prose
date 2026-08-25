def wrap(value):
    return value


class Report:
    def render(self, body, footer):
        return body + footer


class Invoice(Report):
    def render(self, body, footer):
        return wrap(
            super(Invoice, self).render(body,
                                        footer)
        )
