class Report:
    def render(self, body, footer):
        return body


class Invoice(Report):
    def render(self, body, footer):
        return super(Invoice, self).render(body,
                                           footer)
