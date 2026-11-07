import { defineComponent } from 'vue'

import 'virtual:group-icons.css'
import './config-forms.css'

// Holds a configuration block's code group, loading the code-group styles and
// icons with the pages that render one.
export default defineComponent({
  name: 'ConfigForms',
  setup(_, { slots }) {
    return () => slots.default?.()
  }
})
