from mypkg import Base, ColorEnum, Shape

Colors = ColorEnum
Kind = Shape
Node = Base
int_ = int

for color in Colors:
    print(color)

if type(value) == Kind:
    print(Node)

if base is Node:
    raw = int_.from_bytes(payload)
