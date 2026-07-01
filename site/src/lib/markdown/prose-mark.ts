import { findAndReplace } from 'mdast-util-find-and-replace'
import type { Root }      from 'mdast'

import { mdastSpan, mdastText } from './mdast-node'

// Wraps every whole-word Prose or prose in body text in a span.prose-mark
// carrying the captured case. The lookarounds keep Prose-foo and fooProse
// literal, `ignore` leaves headings untouched, and find-and-replace never
// enters code, while link text is still scanned.
const PATTERN = /(?<![\w-])([Pp]rose)(?![\w-])/g

export function remarkProseMark() {
  return (tree: Root): void => {
    findAndReplace(
      tree,
      [PATTERN, (_match, word: string) => mdastSpan('prose-mark', mdastText(word))],
      { ignore: 'heading' }
    )
  }
}
