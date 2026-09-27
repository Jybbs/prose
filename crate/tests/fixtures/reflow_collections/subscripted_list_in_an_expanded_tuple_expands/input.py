def stamp(timetuple):
    return "%s, %02d %s %04d" % (
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][timetuple[6]],
        timetuple[2],
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
         "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][timetuple[1] - 1],
        timetuple[0],
    )
