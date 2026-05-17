import path from 'node:path'
import { fileURLToPath } from 'node:url'

export function repoRoot(metaUrl: string): string {
  return path.resolve(path.dirname(fileURLToPath(metaUrl)), '../../..')
}
