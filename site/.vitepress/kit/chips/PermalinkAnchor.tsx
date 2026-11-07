import { defineComponent } from 'vue'

// Renders the permalink VitePress styles beside a heading.
export const PermalinkAnchor = defineComponent({
  name  : 'PermalinkAnchor',
  props : {
    anchor : { required: true, type: String },
    label  : { required: true, type: String }
  },
  setup(props) {
    return () => <a class="header-anchor" href={`#${props.anchor}`} aria-label={`Permalink to “${props.label}”`}>{'​'}</a>
  }
})
