if __name__ == "__main__":
    selectToken    = CaselessLiteral("select")
    simpleSQL      = selectToken("command") + columnSpec("columns") + fromToken + tableNameList("tables")
