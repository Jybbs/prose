header = parser.Header([
    parser.HeaderLabel([
        parser.ValueTerminal(field_name, "header-name"),
        parser.ValueTerminal(":", "header-sep")]),
    ])
