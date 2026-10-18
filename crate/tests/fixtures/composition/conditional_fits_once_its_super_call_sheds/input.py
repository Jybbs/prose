class Report(Base):

    def summary(self, detailed):
        return super(Report, self).summary_lines() if detailed else self.cached_summary_text
