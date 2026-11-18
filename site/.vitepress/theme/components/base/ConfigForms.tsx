import { defineComponent } from 'vue'

import 'virtual:group-icons.css'

// Holds a configuration block's code group, loading the code-group icons with
// the pages that render one.
export default defineComponent({
  name: 'ConfigForms',
  setup(_, { slots }) {
    return () => slots.default?.()
  }
})
