import { computed, defineComponent } from 'vue'

import { lookup }        from '../../lib/shared/lookup'
import { data as tools } from '../../lib/shared/tools.data'

// Draws a tool's icon alone by its registry slug, hidden from assistive technology.
export const ToolIcon = defineComponent({
  name  : 'ToolIcon',
  props : {
    slug : { required: true, type: String }
  },
  setup(props) {
    const icon = computed(() => lookup(tools.entries, props.slug, 'ToolIcon').icon)

    return () => <svg aria-hidden="true" innerHTML={icon.value.body} viewBox={icon.value.viewBox} />
  }
})
