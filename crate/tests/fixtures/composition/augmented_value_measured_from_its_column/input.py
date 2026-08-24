class Context:
    def measure(self, widget, info):
        if widget:
            if info:
                padx += widget.tk.getint(widget.cget('padx'))
                border += widget.tk.getint(widget.cget('border'))
