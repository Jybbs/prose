import { visitParents } from 'unist-util-visit-parents'
import type { Root }    from 'mdast'

import { pushClassName, withinHeading } from './mdast-node'

// Adds the body-link class to every anchor outside a heading, so authored
// links, autolinks, and the primitive references the wiki-link plugin emits
// share one hover treatment. Registered last so it reaches the links the
// other plugins produce.
export function remarkBodyLink() {
  return (tree: Root): void => {
    visitParents(tree, 'link', (node, ancestors) => {
      if (withinHeading(ancestors)) return
      pushClassName(node, 'body-link')
    })
  }
}
