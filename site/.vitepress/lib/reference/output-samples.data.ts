import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import * as samples         from './output-samples'
import type { LintFinding } from '../fixtures/lint-findings'
import { INPUT_FILE }       from '../fixtures/walker'
import { lintDecorations }  from '../markdown/lint-decorations'
import * as renderer        from '../markdown/renderer'
import { alignedJson }      from '../shared/aligned-json'
import { highlight }        from '../shared/highlight'
import * as paths           from '../shared/paths'

const FENCE: Record<samples.SampleFormat, string> = { github: 'text', json: 'json', text: 'text' }

const root = paths.repoRoot(import.meta.url)

declare const data: samples.OutputSamples
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<samples.OutputSamples> {
    const md     = await renderer.getRenderer()
    const input  = fs.readFileSync(path.join(
      paths.fixturesDirFrom(paths.crateDirFrom(root)), samples.SAMPLE.rule, samples.SAMPLE.case, INPUT_FILE
    ), 'utf8')
    const run    = (args: readonly string[]): string => paths.readProseUnder(root, [
      ...args, '--no-cache', '--color', 'never', '--stdin', '--stdin-filename', samples.SAMPLE.filename
    ], input, '')
    const output = Object.fromEntries(samples.FORMATS.map(format =>
      [format, run(['check', '--output-format', format])])) as Record<samples.SampleFormat, string>
    const json   = samples.jsonRecords<LintFinding & { kind: string }>(output.json)
    const found  = json.filter(record => record.kind === 'diagnostic')
    const blocks = (format: samples.SampleFormat): Promise<string[]> => Promise.all(
      samples.splitRecords(format, output[format]).map(block => renderer.renderFencedHtml(md, block, FENCE[format])))

    return {
      diffHtml  : await renderer.renderFencedHtml(md, run(['format', '--diff']).trimEnd(), 'diff'),
      findings  : await Promise.all(found.map(async finding => ({
        code      : finding.code,
        column    : finding.location.column,
        inputHtml : await highlight(input, 'python', lintDecorations([finding], input)),
        row       : finding.location.row
      }))),
      inputHtml : await highlight(input, 'python'),
      records   : Object.fromEntries(await Promise.all(samples.FORMATS.map(async format =>
        [format, await blocks(format)] as const))) as Record<samples.SampleFormat, string[]>,
      summary   : await renderer.renderFencedHtml(md, alignedJson(json.at(-1)), 'json')
    }
  }
})
