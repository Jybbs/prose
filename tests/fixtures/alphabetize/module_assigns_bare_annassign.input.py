"""
A run mixing bare annotated assignments with valued bindings
reorders normally. Bare `Stmt::AnnAssign` contributes its target
name to the run but has no RHS to walk for dependencies or
side-effect taint.
"""

zebra: int
apple = 1
banana: str = "two"
