import fs   from 'node:fs'
import path from 'node:path'

import { repoRoot, siteDir }    from '../shared/paths'
import { readCargoVersion }     from '../shared/version'
import { loadBrandAssets }      from './assets'
import { enumeratePages }       from './pages'
import { renderCard }           from './render'
import { buildCard }            from './template'

const PROBES: readonly string[] = [
  'rules/align-equals.md',
  'rules/alphabetize.md',
  'rules/collection-layout.md',
  'rules/docstring-wrap.md',
  'rules/loose-constants.md',
  'primitives/source.md',
  'reference/cli.md',
  'integrations/editor.md',
  'usage/installation.md',
  'usage/suppression.md'
]

const outDir = process.argv[2] ?? path.join(repoRoot(import.meta.url), 'og-previews')

async function main(): Promise<void> {
  const srcDir = siteDir(import.meta.url)
  const repo   = repoRoot(import.meta.url)
  const { fonts, glyph, wordmark } = loadBrandAssets(srcDir, repo)
  const version = readCargoVersion(repo)
  fs.mkdirSync(outDir, { recursive: true })
  for (const page of enumeratePages(srcDir, PROBES)) {
    const png  = await renderCard(buildCard(page, version, wordmark, glyph), fonts)
    const dest = path.join(outDir, `${page.slug}.png`)
    fs.writeFileSync(dest, png)
    console.log(`Wrote ${dest} (${png.length} bytes)`)
  }
  console.log(`\nDirectory ${outDir} ready`)
}

await main()
