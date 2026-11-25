class Turtle:
    def stamp(self, screen, item, poly, fc, oc):
        if poly:
            if item:
                screen._drawpoly(item, poly, fill=self._cc(fc),
                                 outline=self._cc(oc), width=self._outlinewidth, top=True)
