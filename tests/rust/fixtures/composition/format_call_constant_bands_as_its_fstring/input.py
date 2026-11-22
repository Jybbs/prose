HEX = "[0-9A-Fa-f]{1,4}"
IPV4 = r"(?:[0-9]{1,3}\.){3}[0-9]{1,3}"
LS32 = "(?:{hex}:{hex}|{ipv4})".format(hex=HEX, ipv4=IPV4)
SUBS = {"hex": HEX, "ls32": LS32}
VARIATIONS = ["(?:%(hex)s:){6}%(ls32)s", "::(?:%(hex)s:){5}%(ls32)s"]
