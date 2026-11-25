def configure(conn):
    if (conn.family in {INET_FAMILY_TAG, INET6_FAMILY_TAG} and
            conn.type == STREAM_KIND and
            conn.proto == TCP_PROTOCOL):
        conn.enable()
