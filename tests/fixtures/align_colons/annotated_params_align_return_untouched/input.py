"""
A multi-line function signature whose parameters are all annotated.
The `:` column aligns across every annotated parameter at the
widest name's width. The return-type colon at the end of the
signature line is not part of the parameter group and stays
untouched.
"""

def dispatch(
    request_id: str,
    user: User,
    priority: int,
    timeout: float,
) -> Response:
    return _impl(request_id, user, priority, timeout)
