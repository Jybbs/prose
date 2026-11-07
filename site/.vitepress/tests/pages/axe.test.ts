import { chromium } from 'playwright'
import type { Browser, BrowserContext } from 'playwright'

import { perScheme, SCHEMES } from '../../lib/shared/palette'
import type { Scheme }        from '../../lib/shared/palette'

import { audit }        from '../axe'
import type { Finding } from '../axe'

import { bundleContext, open, PAGES, WIDE } from './bundle'

let browser  : Browser
let contexts : Record<Scheme, Promise<BrowserContext>>

beforeAll(async () => {
  browser  = await chromium.launch()
  contexts = perScheme(colorScheme => bundleContext(browser, { colorScheme, viewport: WIDE }))
})

afterAll(() => browser.close())

const audits = new Map<string, Record<Scheme, Promise<Finding[]>>>()

// Loads the page once in each scheme, the first test to read it starting both
// loads, and holds what each load found for that scheme's test.
const audited = (file: string): Record<Scheme, Promise<Finding[]>> => audits.getOrInsertComputed(file, () =>
  perScheme(async scheme => open(await contexts[scheme], file, audit)))

it('finds the built pages', () => {
  expect(PAGES, 'the sweep reads the bundle `mise run site:press` builds').toContain('reference/kit.html')
})

describe.each(SCHEMES)('the built pages in the %s scheme', scheme => {
  it.concurrent.each(PAGES)('%s has no axe violations', async file => {
    await expect(audited(file)[scheme]).resolves.toStrictEqual([])
  })
})
