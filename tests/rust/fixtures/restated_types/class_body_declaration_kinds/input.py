from typing import TYPE_CHECKING, ClassVar


class Remote:
    """
    A remote endpoint.

    Attributes:
        registry (ClassVar[dict]): Declared as a class variable.
        cache (dict): Declared inside a type-checking block.
    """

    registry: ClassVar[dict] = {}

    if TYPE_CHECKING:
        cache: dict
