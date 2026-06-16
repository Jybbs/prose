import os         from 'node:os'
import { Worker } from 'node:worker_threads'

import type { BrandAssets } from './assets'
import { landingSvg }       from './landing'
import type { OgPage }      from './pages'
import { svgToPng }         from './parts'
import { pageSvg }          from './template'

interface CardId {
  key        : string
  outputPath : string
}

export interface RenderTask extends CardId { page: OgPage | 'landing' }

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
    svg        : task.page === 'landing'
      ? await landingSvg(brand, version)
      : await pageSvg(task.page, brand, version)
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
  return { key: job.key, outputPath: job.outputPath, png: svgToPng(job.svg) }
}
