// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

const { frontmatter } = vi.hoisted(() => ({
  frontmatter: {
    value: {
      fixtures: {
        'align_equals/basic_run': {
          changesSource : true,
          configHtml    : '<pre><code>max-shift = 0</code></pre>',
          hasFindings   : false,
          hasToggle     : true,
          inputHtml     : '<pre><code>a = 1</code></pre>',
          outputHtml    : '<pre><code>a  = 1</code></pre>',
          sandboxSeed   : { configToml: 'max-shift = 0\n', source: 'a = 1\n' },
          titleHtml     : 'A <code>Run</code> Aligns'
        },
        'align_equals/settled_run': {
          changesSource : false,
          hasFindings   : false,
          hasToggle     : false,
          inputHtml     : '<pre><code>a = 1</code></pre>',
          outputHtml    : '<pre><code>a = 1</code></pre>',
          sandboxSeed   : { configToml: '', source: 'a = 1\n' },
          titleHtml     : 'Settled'
        }
      }
    }
  }
}))

vi.mock('vitepress', async importOriginal => ({
  ...(await importOriginal<typeof import('vitepress')>()),
  useData: () => ({ frontmatter })
}))

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ slug: 'align-equals' }]))

import { Fixture }     from '../../theme/components/fixtures/Fixture'
import { FixtureCard } from '../../theme/components/fixtures/FixtureCard'

const STUBS = { global: { stubs: { FixturePairDoc: true } } }

const card = (caseName = 'basic_run') =>
  mount(FixtureCard, {
    ...STUBS,
    props: { case: caseName, rule: 'align_equals' }
  })

const bare = (caseName = 'basic_run') =>
  mount(Fixture, { ...STUBS, props: { case: caseName, rule: 'align_equals' } })

describe('FixtureCard', () => {
  it('draws its title, its anchor id, and its family', () => {
    const w = card()
    expect(w.get('.fixture-card').attributes('id')).toBe('fixture-align_equals-basic_run')
    expect(w.get('.fixture-card').attributes('data-family')).toBe('alignment')
    expect(w.get('.fixture-card-title').html()).toContain('<code>Run</code>')
  })

  it('opens the body only once the summary row is clicked', async () => {
    const w = card()
    expect(w.get('.fixture-card').attributes('data-open')).toBeUndefined()
    await w.get('.fixture-card-summary-row').trigger('click')
    expect(w.get('.fixture-card').attributes('data-open')).toBe('true')
  })

  it('opens the body once from a press of the summary button', async () => {
    const w = card()
    await w.get('.fixture-card-summary').trigger('click')
    expect(w.get('.fixture-card').attributes('data-open')).toBe('true')
  })

  // `FixtureNoChange` reserves the toggle's footprint with an inert clone, so the
  // interactive tablist is what tells the two apart.
  it('shows the no-change chip in place of the toggle for a settled case', () => {
    expect(card('settled_run').findAll('.fixture-nochange')).toHaveLength(1)
    expect(card('settled_run').findAll('[role="tablist"]')).toHaveLength(0)
    expect(card().findAll('[role="tablist"]')).toHaveLength(1)
  })

  it('shows the configuration a case overrides, and none where it overrides nothing', () => {
    expect(card().get('.fixture-config').html()).toContain('max-shift = 0')
    expect(card('settled_run').findAll('.fixture-config')).toHaveLength(0)
  })

  it('hands the pair the session that opens the case in the sandbox', async () => {
    const w = card()
    await w.get('.fixture-card-summary').trigger('click')
    expect(w.getComponent({ name: 'FixturePairDoc' }).props('sandboxSeed'))
      .toStrictEqual({ configToml: 'max-shift = 0\n', source: 'a = 1\n' })
  })
})

describe('Fixture', () => {
  it('draws the pair alone, with no card chrome and no anchor id', () => {
    const w = bare()
    expect(w.findAll('.fixture-card')).toHaveLength(0)
    expect(w.get('.fixture').attributes('id')).toBeUndefined()
    expect(w.findAll('fixture-pair-doc-stub')).toHaveLength(1)
  })

  it('drops the toggle bar for a case that rewrites nothing', () => {
    expect(bare().findAll('.fixture-bar')).toHaveLength(1)
    expect(bare('settled_run').findAll('.fixture-bar')).toHaveLength(0)
  })

  it('shows the configuration and hands the pair its sandbox session', () => {
    const w = bare()
    expect(w.findAll('.fixture-config')).toHaveLength(1)
    expect(w.getComponent({ name: 'FixturePairDoc' }).props('sandboxSeed').source).toBe('a = 1\n')
  })
})
