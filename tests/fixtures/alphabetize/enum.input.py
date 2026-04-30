"""
Enum members alphabetize as a single group. Detection runs against
the simple name of any base class, so `StrEnum` qualifies the same
way `Enum` and `IntEnum` do.
"""

from enum import StrEnum


class OnetSkillType(StrEnum):
    TECHNOLOGY = "technology"
    ABILITY    = "ability"
    SKILL      = "skill"
    KNOWLEDGE  = "knowledge"
    DWA        = "dwa"
    TASK       = "task"
