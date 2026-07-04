// @vitest-environment happy-dom
import { afterEach, describe, expect, test } from 'vitest'

import { defineTabSelectElement } from '../../../src/lib/shared/dom/tab-select'

let seq = 0

// Registers a fresh custom element (a tag is one-shot in the registry) over a
// tabs-and-panels subtree, connecting it so `wireTabSelect` runs.
function mountTabs(
  markup  : string,
  options : Partial<Parameters<typeof defineTabSelectElement>[0]> = {}
): { host: HTMLElement, panel: (lang: string) => HTMLElement, tab: (lang: string) => HTMLElement } {
  const tag = `x-tabs-${++seq}`
  defineTabSelectElement({ events: ['click'], key: 'lang', panels: '.panel', tabs: '.tab', ...options }, tag)
  const host = document.createElement(tag)
  host.innerHTML = markup
  document.body.append(host)
  return {
    host,
    panel : lang => host.querySelector(`.panel[data-lang="${lang}"]`) as HTMLElement,
    tab   : lang => host.querySelector(`.tab[data-lang="${lang}"]`) as HTMLElement
  }
}

const TABS = `
  <button class="tab" data-lang="py" role="tab">Py</button>
  <button class="tab" data-lang="rs" role="tab">Rs</button>
  <button class="tab all">All</button>
  <div class="panel" data-lang="py">P</div>
  <div class="panel" data-lang="rs">R</div>`

afterEach(() => { document.body.innerHTML = '' })

describe('defineTabSelectElement', () => {
  test('selecting a keyed tab activates it, sets aria, and hides the mismatched panels', () => {
    const { host, panel, tab } = mountTabs(TABS)
    tab('py').dispatchEvent(new Event('click'))

    expect(tab('py')).toHaveClass('is-active')
    expect(tab('py')).toHaveAttribute('aria-selected', 'true')
    expect(tab('rs')).toHaveAttribute('aria-selected', 'false')
    expect(panel('py').hidden).toBe(false)
    expect(panel('rs').hidden).toBe(true)
    expect(host.querySelector('.all')).not.toHaveAttribute('aria-selected')
  })

  test('a tab with no key value shows every panel and skips aria on the roleless tab', () => {
    const { host, panel, tab } = mountTabs(TABS)
    tab('py').dispatchEvent(new Event('click'))
    ;(host.querySelector('.all') as HTMLElement).dispatchEvent(new Event('click'))

    expect(host.querySelector('.all')).toHaveClass('is-active')
    expect(host.querySelector('.all')).not.toHaveAttribute('aria-selected')
    expect(tab('py')).not.toHaveClass('is-active')
    expect(panel('py').hidden).toBe(false)
    expect(panel('rs').hidden).toBe(false)
  })

  test('honors a custom active class and a non-click event', () => {
    const { tab } = mountTabs(TABS, { activeClass: 'chosen', events: ['mouseenter'] })
    tab('rs').dispatchEvent(new Event('mouseenter'))
    expect(tab('rs')).toHaveClass('chosen')
    expect(tab('py')).not.toHaveClass('chosen')
  })
})
