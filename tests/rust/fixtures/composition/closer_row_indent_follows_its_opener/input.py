class Pool:
    def start(self, initializer, initargs):
        (self._create_worker_context,
         self._resolve_work_item_task,
         ) = type(self).prepare_context(initializer, initargs)
