"""
Input that is already collapsed and aligned. The rule emits
zero edits and the output equals the input byte-for-byte.
"""

match credential.kind:
    case "apprenticeship" : icon = "wrench"
    case "certification"  : icon = "scroll"
    case "program"        : icon = "cap"
