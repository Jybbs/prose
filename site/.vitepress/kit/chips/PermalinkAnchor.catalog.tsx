import type { Declaration } from '../catalog'
import { PermalinkAnchor }  from './PermalinkAnchor'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'Beside a line, shown on hover',
      render : () => (
        <p class="m-0 font-mono text-sm">
          kit
          <PermalinkAnchor anchor="permalinkanchor" label="PermalinkAnchor" />
        </p>
      )
    }
  ],
  use      : 'Renders the link to a heading that the page styles beside each heading, named for the heading it opens.'
}
