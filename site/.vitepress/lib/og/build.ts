import fs   from 'node:fs'
import path from 'node:path'

import { readCargoVersion } from '../shared/version'
import { loadBrandAssets }  from './assets'
import { renderLanding }    from './landing'
import { enumeratePages }   from './pages'
import { renderPage }       from './render'

export async function buildOgCards(
  srcDir : string,
  pages  : readonly string[],
  outDir : string
): Promise<void> {
  const repo    = path.dirname(srcDir)
  const brand   = loadBrandAssets(srcDir)
  const version = readCargoVersion(repo)
  fs.writeFileSync(path.join(outDir, 'og.png'), await renderLanding(brand, version))
  await Promise.all(
    enumeratePages(srcDir, pages).map(async page => {
      const dest = path.join(outDir, page.outputPath)
      fs.mkdirSync(path.dirname(dest), { recursive: true })
      fs.writeFileSync(dest, await renderPage(page, brand, version))
    })
  )
}
