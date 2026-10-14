import os         from 'node:os'
import { Worker } from 'node:worker_threads'

import { Resvg } from '@resvg/resvg-js'

import type { BrandAssets } from './assets'
import { iconSvg }          from './icon'
import { landingSvg }       from './landing'
import type { OgPage }      from '../pages'
import { pageSvg }          from './template'

interface CardId {
  key        : string
  outputPath : string
}

export type Card = OgPage | 'icon' | 'landing'

export interface RenderTask extends CardId { card: Card }

interface RasterJob extends CardId { svg: string }

interface RenderedCard extends CardId { png: Uint8Array }

const WORKER_URL = new URL('./resvg-worker.mjs', import.meta.url)

// satori (text-to-vector) runs here, leaving the resvg rasterization to fan out across cores
export async function renderCards(
  brand   : BrandAssets,
  version : string,
  tasks   : readonly RenderTask[]
): Promise<readonly RenderedCard[]> {
  const jobs = await Promise.all(tasks.map(async task => ({
    key        : task.key,
    outputPath : task.outputPath,
    svg        : await cardSvg(brand, task.card, version)
  })))

  try {
    const lanes   = Math.min(jobs.length, Math.max(1, os.availableParallelism() - 1))
    const batches = await Promise.all(partition(jobs, lanes).map(lane => runLane(lane)))
    return batches.flat()
  }
  catch {
    // a runtime without worker support still rasterizes every card, serially
    return jobs.map(rasterize)
  }
}

function cardSvg(brand: BrandAssets, card: Card, version: string): Promise<string> {
  if (card === 'icon')    return iconSvg(brand)
  if (card === 'landing') return landingSvg(brand, version)
  return pageSvg(card, brand, version)
}

function partition<T>(items: readonly T[], lanes: number): T[][] {
  const buckets: T[][] = Array.from({ length: lanes }, () => [])
  items.forEach((item, index) => buckets[index % lanes].push(item))
  return buckets
}

function runLane(jobs: readonly RasterJob[]): Promise<readonly RenderedCard[]> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(WORKER_URL, { workerData: { jobs } })
    worker.once('message', cards => { void worker.terminate(); resolve(cards as RenderedCard[]) })
    worker.once('error',   error => { void worker.terminate(); reject(error) })
  })
}

function rasterize(job: RasterJob): RenderedCard {
  // satori embeds every glyph as a vector path, so resvg needs no font lookup
  const png = new Resvg(job.svg, { font: { loadSystemFonts: false } }).render().asPng()
  return { key: job.key, outputPath: job.outputPath, png }
}
