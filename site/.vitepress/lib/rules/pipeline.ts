import { execFileSync } from 'node:child_process'

import { repoRoot, resolveProseBinary } from '../shared/paths'
import { requireString }                from '../shared/require-string'

interface PipelineEntry {
  imperative : string
  position   : number
  slug       : string
}

export function parsePipelineJson(text: string): readonly PipelineEntry[] {
  const parsed: unknown = JSON.parse(text)
  if (!Array.isArray(parsed) || parsed.length === 0) {
    throw new Error('prose rules emitted no pipeline entries')
  }
  return parsed.map((entry, i) => {
    const { imperative, position, slug } = entry as Partial<PipelineEntry>
    if (typeof position !== 'number') {
      throw new Error(`pipeline entry ${i} has invalid or missing position`)
    }
    return {
      imperative : requireString(imperative, `pipeline entry ${i} has invalid or missing imperative`),
      position,
      slug       : requireString(slug, `pipeline entry ${i} has invalid or missing slug`)
    }
  })
}

export function readPipeline(metaUrl: string): readonly PipelineEntry[] {
  const binary = resolveProseBinary(repoRoot(metaUrl))
  const output = execFileSync(binary, ['rules', '--output-format', 'json'], { encoding: 'utf8' })
  return parsePipelineJson(output)
}
