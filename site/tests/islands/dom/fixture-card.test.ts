// @vitest-environment happy-dom
import { describe, expect, test } from 'vitest'

import { setFixtureCardOpen } from '../../../src/lib/fixtures/card'

const card = (withSummary: boolean): HTMLElement => {
  const root = document.createElement('div')
  root.className = 'fixture-card'
  if (withSummary) {
    const summary = document.createElement('button')
    summary.className = 'fixture-card-summary'
    root.append(summary)
  }
  return root
}

describe('setFixtureCardOpen', () => {
  test('opens the card and marks the summary expanded', () => {
    const root = card(true)
    setFixtureCardOpen(root, true)
    expect(root).toHaveClass('is-open')
    expect(root.querySelector('.fixture-card-summary')).toHaveAttribute('aria-expanded', 'true')
  })

  test('closes the card and clears the expanded flag', () => {
    const root = card(true)
    setFixtureCardOpen(root, true)
    setFixtureCardOpen(root, false)
    expect(root).not.toHaveClass('is-open')
    expect(root.querySelector('.fixture-card-summary')).toHaveAttribute('aria-expanded', 'false')
  })

  test('toggles the class even when no summary is present', () => {
    const root = card(false)
    expect(() => setFixtureCardOpen(root, true)).not.toThrow()
    expect(root).toHaveClass('is-open')
  })
})
