import type { Declaration } from '../catalog'
import { ScrollRegion }     from './ScrollRegion'

export const declaration: Declaration = {
  examples : [
    { label: 'Preformatted', render: () => <ScrollRegion as="pre" class="overflow-x-auto">{{ default: () => 'prose --check src/' }}</ScrollRegion> }
  ],
  use      : 'Gives an element that scrolls a tab stop, so a keyboard scrolls it at any width.'
}
