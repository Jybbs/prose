import { defineComponent } from 'vue'

import { PermalinkAnchor } from './PermalinkAnchor'

// Renders a section's `h2` beside its permalink.
export const DocHeading = defineComponent({
  name  : 'DocHeading',
  props : {
    id    : { required: true, type: String },
    title : { required: true, type: String }
  },
  setup(props) {
    return () => (
      <h2 id={props.id} tabindex="-1">
        {props.title}{' '}
        <PermalinkAnchor anchor={props.id} label={props.title} />
      </h2>
    )
  }
})
