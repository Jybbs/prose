class Dispatcher:
    def handle_zeta(self, value):
        return value * 2
    def handle_alpha(self, value):
        match value:
            case 1:
                return "one"
            case 2:
                return "two"
            case _:
                return "other"
    def handle_beta(self, value):
        return value + 1
