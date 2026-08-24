class ForkServer:
    def ensure_running(self):
        try:
            fds_to_pass = [listener.fileno(), alive_r, authkey_r]
            main_kws["authkey_r"] = authkey_r
            cmd %= (listener.fileno(), alive_r, self._preload_modules,
                    main_kws)
            exe = spawn.get_executable()
            args = [exe] + util._args_from_interpreter_flags()
        finally:
            pass
