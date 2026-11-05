import type { Declaration } from '../catalog'
import { MiddleEllipsis }   from './MiddleEllipsis'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'A path too wide for its box',
      render : () => (
        <div class="flex w-160 font-mono text-sm">
          <MiddleEllipsis tail={8} text="crate/src/rules/alignment/align_equals.rs" />
        </div>
      )
    }
  ],
  use      : 'Shortens a path from its middle until it fits its box, keeping the end of the path readable.'
}
