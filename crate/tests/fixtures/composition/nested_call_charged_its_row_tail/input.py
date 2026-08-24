class Widget:
    def configure(self, cmd, cnf):
        self.tk.call(_flatten((self._w, cmd)) + self._options(cnf))
