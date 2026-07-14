import type { DecorationItem } from '@shikijs/types'

// Renders the code in a shiki-shaped `pre`, appending a lint flag per
// decoration so squiggle assertions can read `data-rule` off the output.
export function highlight(
  code        : string,
  _lang       : string,
  decorations : readonly DecorationItem[] = []
): Promise<string> {
  const flags = decorations
    .map(item => `<span class="lint-flag" data-rule="${item.properties?.['data-rule']}">x</span>`)
    .join('')
  return Promise.resolve(`<pre class="shiki"><code>${code}${flags}</code></pre>`)
}
