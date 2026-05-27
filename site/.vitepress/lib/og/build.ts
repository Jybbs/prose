import fs   from 'node:fs'
import path from 'node:path'

import { readCargoVersion }     from '../shared/version'
import { loadBrandAssets }      from './assets'
import { enumeratePages }       from './pages'
import { renderCard }           from './render'
import { buildCard }            from './template'

export async function buildOgCards(srcDir: string, pages: readonly string[], outDir: string): Promise<void> {
  const repo = path.dirname(srcDir)
  const { fonts, glyph, wordmark } = loadBrandAssets(srcDir, repo)
  const version = readCargoVersion(repo)
  for (const page of enumeratePages(srcDir, pages)) {
    const dest = path.join(outDir, page.outputPath)
    fs.mkdirSync(path.dirname(dest), { recursive: true })
    const png = await renderCard(buildCard(page, version, wordmark, glyph), fonts)
    fs.writeFileSync(dest, png)
  }
}
