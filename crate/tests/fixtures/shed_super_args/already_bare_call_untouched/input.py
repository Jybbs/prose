class Widget:
    def render(self):
        return ""


class Button(Widget):
    def render(self):
        return super().render() + "!"
