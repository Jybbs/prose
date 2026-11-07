import { alignedJson } from '../shared/aligned-json'

export const FORMATS = ['text', 'json', 'github'] as const

// The fixture case every sample runs over, and the path its stdin run names.
export const SAMPLE = { case: 'varying_lhs_widths', filename: 'src/configure.py', rule: 'align_equals' } as const

export interface OutputSamples {
  diffHtml  : string
  findings  : readonly SampleFinding[]
  inputHtml : string
  records   : Record<SampleFormat, readonly string[]>
  summary   : string
}

interface SampleFinding {
  code      : string
  column    : number
  inputHtml : string
  row       : number
}

export type SampleFormat = (typeof FORMATS)[number]

// Parses each line of a `json` run as its record, the closing summary record
// included.
export function jsonRecords<T = { kind: string }>(output: string): T[] {
  return output.trimEnd().split('\n').map(line => JSON.parse(line) as T)
}

// Splits one format's output into a block per diagnostic, in the order the
// `json` records arrive, leaving the closing summary record out.
export function splitRecords(format: SampleFormat, output: string): string[] {
  if (format === 'json') {
    return jsonRecords(output).filter(record => record.kind !== 'summary').map(record => alignedJson(record))
  }
  const trimmed = output.trimEnd()
  return format === 'github' ? trimmed.split('\n') : trimmed.split(/\n(?=(?:warning|error): )/)
}
