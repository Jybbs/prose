import type { Root } from 'mdast'

import { pushClassName, visitOutsideHeadings } from './mdast-node'

// Registered last so authored links, autolinks, and the anchors the earlier
// plugins emit all share one hover treatment.
export function remarkBodyLink() {
  return (tree: Root): void => {
    visitOutsideHeadings(tree, 'link', node => pushClassName(node, 'body-link'))
  }
}
