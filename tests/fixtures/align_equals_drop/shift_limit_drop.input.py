"""
Under the `drop` policy, an outlier is excluded from padding math
but the group stays logically intact. Members above and below the
outlier align against one shared `=` column, as if the outlier were
invisible. The outlier itself keeps its original spacing.
"""

short = 1
medium_size = 2
really_really_long_target = 3
longer = 4
tiny = 5
