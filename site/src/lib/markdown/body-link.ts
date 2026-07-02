import { visitParents } from 'unist-util-visit-parents'
import type { Root }    from 'mdast'

import { pushClassName, withinHeading } from './mdast-node'

// Registered last so authored links, autolinks, and the anchors the earlier
// plugins emit all share one hover treatment.
export function remarkBodyLink() {
  return (tree: Root): void => {
    visitParents(tree, 'link', (node, ancestors) => {
      if (withinHeading(ancestors)) return
      pushClassName(node, 'body-link')
    })
  }
}
