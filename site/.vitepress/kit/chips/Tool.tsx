import { computed, defineComponent } from 'vue'

import { lookup }        from '../../lib/shared/lookup'
import { data as tools } from '../../lib/shared/tools.data'

// Links to a tool by its registry slug, drawing its icon beside its name, or
// draws the icon alone with no link where `bare` is set.
export const Tool = defineComponent({
  name  : 'Tool',
  props : {
    bare : Boolean,
    slug : { required: true, type: String }
  },
  setup(props) {
    const entry = computed(() => lookup(tools.entries, props.slug, 'Tool'))

    return () => {
      const { body, viewBox } = entry.value.icon
      return props.bare
        ? <svg aria-hidden="true" class="tool-mark-icon inline-block" innerHTML={body} viewBox={viewBox} />
        : (
          <a class="tool-mark" href={entry.value.href} rel="noopener" target="_blank" title={entry.value.name}>
            <code><svg aria-hidden="true" class="tool-mark-icon mr-3 inline-block size-0-78em" innerHTML={body} viewBox={viewBox} />{entry.value.name}</code>
          </a>
        )
    }
  }
})
