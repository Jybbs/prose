def drain(reader):
    while (chunk := reader.read()) != b"":
        handle(chunk)
