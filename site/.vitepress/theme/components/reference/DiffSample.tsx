import { defineComponent } from 'vue'

import { data } from '../../../lib/reference/output-samples.data'

// Draws the patch `prose format --diff` prints for the output samples' file.
export const DiffSample = defineComponent({
  name: 'DiffSample',
  setup() {
    return () => <div innerHTML={data.diffHtml} />
  }
})
