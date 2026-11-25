class Conf:
    def GetOption(self, configType, section, option, raw):
        try:
            pass
        except ValueError:
            warning = ('\n Warning: config.py - IdleConf.GetOption -\n'
                       ' invalid %r value for configuration option %r\n'
                       ' from section %r: %r' %
                       (type, option, section,
                       self.userCfg[configType].Get(section, option, raw=raw)))
