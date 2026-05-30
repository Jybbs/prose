import fs   from 'node:fs'
import path from 'node:path'

import { readCargoVersion }                            from '../shared/version'
import { loadBrandAssets }                             from './assets'
import { cardKeyer, pruneCards, readCard, writeCard }  from './cache'
import { renderLanding }                               from './landing'
import { enumeratePages }                              from './pages'
import { renderPage }                                  from './template'

interface CardJob {
  key        : string
  outputPath : string
  render     : () => Promise<Buffer>
}

export async function buildOgCards(
  srcDir : string,
  pages  : readonly string[],
  outDir : string
): Promise<void> {
  const repo     = path.dirname(srcDir)
  const brand    = loadBrandAssets(srcDir)
  const version  = readCargoVersion(repo)
  const cacheDir = path.join(repo, '.cache', 'og')
  const keyOf    = cardKeyer(version, brand)

  const jobs: readonly CardJob[] = [
    { key: keyOf('landing'), outputPath: 'og.png', render: () => renderLanding(brand, version) },
    ...enumeratePages(srcDir, pages).map(page => ({
      key        : keyOf(page),
      outputPath : page.outputPath,
      render     : () => renderPage(page, brand, version)
    }))
  ]

  await Promise.all(jobs.map(job => materialize(cacheDir, outDir, job)))
  await pruneCards(cacheDir, jobs.map(job => job.key))
}

async function materialize(cacheDir: string, outDir: string, job: CardJob): Promise<void> {
  const png  = await readCard(cacheDir, job.key) ?? await renderAndCache(cacheDir, job)
  const dest = path.join(outDir, job.outputPath)
  fs.mkdirSync(path.dirname(dest), { recursive: true })
  fs.writeFileSync(dest, png)
}

async function renderAndCache(cacheDir: string, job: CardJob): Promise<Buffer> {
  const png = await job.render()
  await writeCard(cacheDir, job.key, png)
  return png
}
