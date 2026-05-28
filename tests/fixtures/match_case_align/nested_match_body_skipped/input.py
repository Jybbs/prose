"""
An outer match with one arm whose body is itself a `match` and
one arm whose body is a single assignment. The nested-match arm
disqualifies and stays multi-line. The second arm is alone in
its sub-group and collapses without padding. The inner match is
visited independently and collapses-and-aligns its three
single-statement arms.
"""

match outer:
    case "wrap":
        match inner:
            case "alpha":
                value = 1
            case "beta":
                value = 2
            case "gamma":
                value = 3
    case "skip":
        value = 0
