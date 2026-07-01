import { h } from 'hastscript'

import { ExpressiveCodeAnnotation }    from '@expressive-code/core'
import type { AnnotationRenderOptions } from '@expressive-code/core'
import type {
  ExpressiveCodeBlock,
  ExpressiveCodeInlineRange,
  ExpressiveCodeLine,
  ExpressiveCodePlugin
} from '@expressive-code/core'
import type { Parents, Properties } from 'hast'

import type { LintFinding } from '../content/schemas'

// Wraps a flagged span in a `.lint-flag` element carrying the finding's hover
// payload as `data-*`, the hooks the tooltip layer reads.
class LintFlagAnnotation extends ExpressiveCodeAnnotation {
  constructor(private readonly finding: LintFinding, inlineRange: ExpressiveCodeInlineRange) {
    super({ inlineRange })
  }

  render({ nodesToTransform }: AnnotationRenderOptions): Parents[] {
    const attrs = this.dataset()
    return nodesToTransform.map(node => h('span.lint-flag', attrs, node))
  }

  private dataset(): Properties {
    const data: Properties = { 'data-message': this.finding.message, 'data-rule': this.finding.code }
    const edit = this.finding.fix?.edits[0]
    if (edit) {
      data['data-before']    = edit.before
      data['data-suggested'] = edit.content
    }
    return data
  }
}

// Splits a finding into one range per line it spans, the first from its start
// column to the line end, the interior lines whole, and the last to its end
// column, dropping the 1-indexed harness positions to 0-indexed.
function* findingRanges(
  finding: LintFinding,
  block: ExpressiveCodeBlock
): Iterable<{ line: ExpressiveCodeLine, range: ExpressiveCodeInlineRange }> {
  const firstLine = finding.location.row - 1
  const lastLine  = finding.end_location.row - 1
  for (let lineIndex = firstLine; lineIndex <= lastLine; lineIndex++) {
    const line = block.getLine(lineIndex)
    if (!line) continue
    yield {
      line,
      range: {
        columnEnd   : lineIndex === lastLine  ? finding.end_location.column - 1 : line.text.length,
        columnStart : lineIndex === firstLine ? finding.location.column - 1     : 0
      }
    }
  }
}

// The Expressive Code plugin that decorates a `lint="<rule>/<case>"` fence with
// its harness findings, read from the build-time map the config binds. An id
// the map does not hold is a build error, since it names a fixture with no
// findings to draw.
export function pluginLintFlag(findings: Map<string, LintFinding[]>): ExpressiveCodePlugin {
  return {
    name: 'prose:lint-flag',
    hooks: {
      preprocessMetadata({ codeBlock }) {
        const id = codeBlock.metaOptions.getString('lint')
        if (id === undefined) return
        const found = findings.get(id)
        if (!found) throw new Error(`lint="${id}" references no fixture findings`)
        for (const finding of found) {
          for (const { line, range } of findingRanges(finding, codeBlock)) {
            line.addAnnotation(new LintFlagAnnotation(finding, range))
          }
        }
      }
    }
  }
}
