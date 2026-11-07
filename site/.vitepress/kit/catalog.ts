import type { VNodeChild } from 'vue'

import { CATALOG_SUFFIX } from '../lib/shared/constants'
import { stripSuffix }    from '../lib/shared/strip-suffix'

interface Example {
  label  : string
  render : () => VNodeChild
}

export interface Declaration {
  examples : readonly Example[]
  use      : string
}

const sidecars = import.meta.glob<Declaration>('./**/*.catalog.tsx', { eager: true, import: 'declaration' })

// Lists each sidecar's declaration under the name of the piece it sits beside,
// in the order the catalog page sets them.
export const declarations = new Map(
  Object.entries(sidecars)
    .map(([file, declaration]) => [stripSuffix(file.split('/').pop()!, CATALOG_SUFFIX), declaration] as const)
    .toSorted(([a], [b]) => a.localeCompare(b))
)
