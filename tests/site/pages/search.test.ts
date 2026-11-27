import { chromium }                           from 'playwright'
import type { Browser, BrowserContext, Page } from 'playwright'

import { perScheme, SCHEMES } from '../../../site/.vitepress/lib/shared/palette'
import type { Scheme }        from '../../../site/.vitepress/lib/shared/palette'

import { audit } from '../axe'

import { bundleContext, open, WIDE } from './bundle'

let browser : Browser
beforeAll(async () => { browser = await chromium.launch({ args: ['--mute-audio'] }) })
afterAll(() => browser.close())

const AUDITS = [
  ['align', '.VPLocalSearchBox'],
  ['configuration', '.VPLocalSearchBox'],
  ['align', '#localsearch-list']
] as const

const search = async (page: Page, query: string) => {
  await page.locator('.VPNavBarSearchButton').click()
  await page.locator('.VPLocalSearchBox input').fill(query)
  await page.locator('.VPLocalSearchBox mark').first().waitFor()
}

describe('search results', () => {
  let contexts : Record<Scheme, Promise<BrowserContext>>
  beforeAll(() => {
    contexts = perScheme(colorScheme => bundleContext(browser, { colorScheme, viewport: WIDE }))
  })

  describe.each(SCHEMES)('in the %s scheme', scheme => {
    it.for(AUDITS)('has no violation on %s under %s', async ([query, scope]) => {
      await open(await contexts[scheme], 'index.html', async page => {
        await search(page, query)
        await expect(audit(page, { scope })).resolves.toStrictEqual([])
      })
    })
  })

  it('has no violation on the detailed list', async () => {
    await open(await bundleContext(browser, { viewport: WIDE }), 'index.html', async page => {
      await search(page, 'configuration')
      await page.locator('.VPLocalSearchBox .toggle-layout-button').click()
      await page.locator('.VPLocalSearchBox .excerpt mark').first().waitFor()
      await expect(audit(page, { scope: '.VPLocalSearchBox' })).resolves.toStrictEqual([])
    })
  })

  it('names each result by its text and nests no control', async () => {
    await open(await contexts.light, 'index.html', async page => {
      await search(page, 'configuration')
      const count = await page.locator('.VPLocalSearchBox .results > li').count()
      expect(count).toBeGreaterThan(5)
      const options = page.getByRole('option')
      await expect(options.count()).resolves.toBe(count)
      const texts = await page.locator('a.result').evaluateAll(links => links.map(link => (link as HTMLElement).innerText))
      const client = await page.context().newCDPSession(page)
      const { nodes } = await client.send('Accessibility.getFullAXTree')
      const strip = (text: string) => text.replaceAll(/\s+/g, '')
      const names = nodes.filter(node => node.role?.value === 'option').map(node => node.name?.value as string)
      expect(names.map(strip)).toStrictEqual(texts.map(strip))
    })
  })

  it('moves the selection and opens a result', async () => {
    await open(await contexts.light, 'index.html', async page => {
      await search(page, 'align')
      const active = () => page.evaluate(() => {
        const id = document.querySelector('#localsearch-input')!.getAttribute('aria-activedescendant')!
        const all = document.querySelectorAll(`[id="${id}"]`)
        const selected = document.querySelectorAll('.VPLocalSearchBox [aria-selected="true"]').length
        return [id, all.length, all[0]?.getAttribute('role'), all[0]?.getAttribute('aria-selected'), selected]
      })
      await expect.poll(active).toStrictEqual(['localsearch-item-0', 1, 'option', 'true', 1])
      await page.keyboard.press('ArrowDown')
      await page.keyboard.press('ArrowDown')
      await expect.poll(active).toStrictEqual(['localsearch-item-2', 1, 'option', 'true', 1])
      await page.locator('.VPLocalSearchBox input').fill('align-c')
      await page.waitForTimeout(800)
      await expect.poll(active).toStrictEqual(['localsearch-item-0', 1, 'option', 'true', 1])
      await expect(audit(page, { scope: '.VPLocalSearchBox' })).resolves.toStrictEqual([])
      await page.keyboard.press('ArrowDown')
      const href = await page.locator('a.result.selected').getAttribute('href')
      await page.keyboard.press('Enter')
      await expect.poll(() => new URL(page.url()).pathname + new URL(page.url()).hash).toBe(href)
      await search(page, 'align')
      const next = await page.locator('a.result').nth(3).getAttribute('href')
      await page.locator('a.result').nth(3).click()
      await expect.poll(() => new URL(page.url()).pathname + new URL(page.url()).hash).toBe(next)
      await expect(page.locator('.VPLocalSearchBox').count()).resolves.toBe(0)
    })
  })

  it('has no violation in a box opened again after Escape', async () => {
    await open(await contexts.light, 'index.html', async page => {
      await search(page, 'align')
      await page.keyboard.press('Escape')
      await expect.poll(() => page.locator('.VPLocalSearchBox').count()).toBe(0)
      await search(page, 'configuration')
      await expect(page.locator('#localsearch-list').getByRole('option').count()).resolves.toBeGreaterThan(5)
      await expect(audit(page, { scope: '.VPLocalSearchBox' })).resolves.toStrictEqual([])
    })
  })

  it('shows the empty state', async () => {
    await open(await contexts.light, 'index.html', async page => {
      await search(page, 'align')
      await page.locator('.VPLocalSearchBox input').fill('zzzzqqq')
      await page.locator('.no-results').waitFor()
      await expect(audit(page, { scope: '.VPLocalSearchBox' })).resolves.toStrictEqual([])
    })
  })
})

