result = (
    pstats.Stats(profile)
          .strip_dirs()
          .sort_stats(order)
          .print_stats()
)
