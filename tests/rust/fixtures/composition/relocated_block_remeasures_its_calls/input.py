class Style:
    def element_create(self, class_name, part_id, statemap):
        specs = (class_name, part_id, tuple(_mapdict_values(statemap)))
