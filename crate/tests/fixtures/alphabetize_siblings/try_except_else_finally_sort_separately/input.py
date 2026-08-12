try:
    from zeta import a
    from alpha import b
except ValueError:
    from omega import c
    from delta import d
except KeyError:
    from gamma import e
    from beta import f
else:
    from sigma import g
    from rho import h
finally:
    from psi import i
    from phi import j
