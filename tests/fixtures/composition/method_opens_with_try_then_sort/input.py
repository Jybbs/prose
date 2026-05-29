class Validator:
    def validate_zeta(self, value):
        return value > 0
    def validate_alpha(self, value):
        try:
            int(value)
        except ValueError:
            return False
        return True
    def validate_beta(self, value):
        return isinstance(value, str)
