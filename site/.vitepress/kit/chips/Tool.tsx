import { computed, defineComponent } from 'vue'

import { externalAttrs } from '../../lib/shared/links'
import { lookup }        from '../../lib/shared/lookup'
import { data as tools } from '../../lib/shared/tools.data'
import { ToolIcon }      from './ToolIcon'

// Links to a tool by its registry slug, drawing its icon beside its name.
export const Tool = defineComponent({
  name  : 'Tool',
  props : {
    slug : { required: true, type: String }
  },
  setup(props) {
    const entry = computed(() => lookup(tools.entries, props.slug, 'Tool'))

    return () => (
      <a class="tool-mark" href={entry.value.href} title={entry.value.name} {...externalAttrs(entry.value.href)}>
        <code><ToolIcon class="tool-mark-icon mr-3 inline-block size-0-78em" slug={props.slug} />{entry.value.name}</code>
      </a>
    )
  }
})
