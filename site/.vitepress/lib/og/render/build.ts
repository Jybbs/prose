import fs   from 'node:fs'
import path from 'node:path'

import { ogImagePath }                                 from '../../config/og-url'
import { readCargoVersion }                            from '../../shared/version'
import { loadBrandAssets }                             from './assets'
import { cardKeyer, pruneCards, readCard, writeCard }  from './cache'
import { enumeratePages }                              from '../pages'
import { renderCards, type RenderTask }                from './pool'

export async function buildOgCards(
  srcDir : string,
  pages  : readonly string[],
  outDir : string
): Promise<void> {
  const repo     = path.dirname(srcDir)
  const brand    = loadBrandAssets(srcDir)
  const version  = readCargoVersion(path.join(repo, 'crate'))
  const cacheDir = path.join(repo, '.cache', 'og')
  const keyOf    = cardKeyer(version, brand)

  const tasks: readonly RenderTask[] = [
    { key: keyOf('landing'), outputPath: ogImagePath('index.md'), page: 'landing' },
    ...enumeratePages(srcDir, pages).map(page => ({
      key        : keyOf(page),
      outputPath : page.outputPath,
      page
    }))
  ]

  const misses: RenderTask[] = []
  await Promise.all(tasks.map(async task => {
    const png = await readCard(cacheDir, task.key)
    if (png) writeDist(outDir, task.outputPath, png)
    else misses.push(task)
  }))

  if (misses.length > 0) {
    const rendered = await renderCards(brand, version, misses)
    await Promise.all(rendered.map(async card => {
      writeDist(outDir, card.outputPath, card.png)
      await writeCard(cacheDir, card.key, Buffer.from(card.png))
    }))
  }

  await pruneCards(cacheDir, tasks.map(task => task.key))
}

function writeDist(outDir: string, outputPath: string, png: Uint8Array): void {
  const dest = path.join(outDir, outputPath)
  fs.mkdirSync(path.dirname(dest), { recursive: true })
  fs.writeFileSync(dest, png)
}
