import type { Declaration } from '../catalog'
import { CopyButton }       from './CopyButton'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'Over a command',
      render : () => (
        <div class="copy-host relative rounded-sm border-1 border-divider bg-bg py-8 pr-44 pl-14 font-mono text-sm">
          <code>uv tool install prose</code>
          <CopyButton label="Copy the command" source="uv tool install prose" />
        </div>
      )
    }
  ],
  use      : 'Copies a string to the clipboard from the corner of a block, its glyph and its label reporting the copy.'
}
