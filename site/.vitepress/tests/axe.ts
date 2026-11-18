import axe           from 'axe-core'
import { JSDOM }     from 'jsdom'
import type { Page } from 'playwright'

export interface Finding {
  id      : string
  targets : string[]
}

export const AXE_TIMEOUT_MS = 15_000

let shared: JSDOM | undefined

// Builds the test file's one JSDOM on the first call, with the axe source
// evaluated into it, and gives every later check the same one, each check
// replacing the body the one before it rendered.
function axeDom(): JSDOM {
  if (!shared) {
    shared = new JSDOM('<!DOCTYPE html><body></body>', { runScripts: 'outside-only' })
    shared.window.eval(axe.source)
  }
  return shared
}

// Runs `axe-core` on the page, limited to the rules `runOnly` names where it names any.
export async function audit(page: Page, runOnly?: string[]): Promise<Finding[]> {
  await page.addScriptTag({ content: axe.source })
  return page.evaluate(async options => {
    const { violations } = await (window as unknown as { axe: typeof axe }).axe.run(document, options)
    return violations.map(({ id, nodes }) => ({ id, targets: nodes.map(node => node.target.join(' ')) }))
  }, runOnly ? { runOnly } : {})
}

export async function expectAccessible(
  html   : string,
  ignore : readonly string[] = []
): Promise<void> {
  const { window } = axeDom()
  window.document.body.innerHTML = html
  const run = await (window as unknown as { axe: typeof axe }).axe.run(window.document.body, {
    rules: {
      'color-contrast' : { enabled: false },
      region           : { enabled: false }
    }
  })
  // Copies the violations out of the JSDOM realm axe runs in, whose array
  // prototype differs from this realm's and fails a strict compare.
  expect([...run.violations].filter(v => !ignore.includes(v.id))).toStrictEqual([])
}

export function rendersAccessibly(html: () => string): void {
  it('renders with no axe violations', async () => {
    await expectAccessible(html())
  }, AXE_TIMEOUT_MS)
}
