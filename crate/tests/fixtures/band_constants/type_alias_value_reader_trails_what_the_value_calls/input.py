import annotationlib
from typing import Annotated


def mid():
    return 1


def adapt(alias):
    return alias.__value__


type Scored = Annotated[int, mid()]
VALUE = Scored.__value__
EVALUATED = Scored.evaluate_value(annotationlib.Format.VALUE)
ADAPTED = adapt(Scored)
