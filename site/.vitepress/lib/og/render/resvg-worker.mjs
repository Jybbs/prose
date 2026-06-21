import { parentPort, workerData } from 'node:worker_threads'

import { Resvg } from '@resvg/resvg-js'

// satori embeds every glyph as a vector path, so resvg needs no font lookup
const NO_FONTS = { font: { loadSystemFonts: false } }

const cards = workerData.jobs.map(job => ({
  key        : job.key,
  outputPath : job.outputPath,
  png        : new Resvg(job.svg, NO_FONTS).render().asPng()
}))

parentPort.postMessage(cards)
