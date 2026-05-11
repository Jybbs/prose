"""
A module-level def following the `if __name__ == "__main__":` block
carries 1 blank line of separation. The main-guard case takes
precedence over the 2-blank def-after-stmt case.
"""


if __name__ == "__main__":
    main()



def cleanup():
    return None
