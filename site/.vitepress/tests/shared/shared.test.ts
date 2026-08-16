import { fc, test } from '@fast-check/vitest'

import { railPaint }        from '../../lib/shared/family-rail'
import { inlineCode }       from '../../lib/shared/inline-code'
import { externalAttrs }    from '../../lib/shared/links'
import { lookup }           from '../../lib/shared/lookup'
import { formatFolio }      from '../../lib/shared/numerals'
import { requireString }    from '../../lib/shared/require-string'
import { compositionRoute } from '../../lib/shared/routes'
import { ruleSlug }         from '../../lib/shared/rule-slug'
import { stripSuffix }      from '../../lib/shared/strip-suffix'
import { parseSvg }         from '../../lib/shared/svg'
import { toTitleCase }      from '../../lib/shared/title-case'
import { withFallback }     from '../../lib/shared/with-fallback'

import { warnTest } from '../support'

describe('toTitleCase', () => {
  it.each([
    ['align_equals',    '_', 'Align Equals'],
    ['one-two-the-end', '-', 'One Two the End']
  ])('title-cases %s across its %s separator', (slug, separator, expected) => {
    expect(toTitleCase(slug, separator)).toBe(expected)
  })
})

describe('compositionRoute', () => {
  it('builds the composition page route under the rules section', () => {
    expect(compositionRoute()).toBe('/rules/composition/')
  })
})

describe('ruleSlug', () => {
  it.each([
    ['alphabetize_siblings', 'alphabetize-siblings'],
    ['align-equals',         'align-equals']
  ])('converts %s to %s', (fixtureRule, expected) => {
    expect(ruleSlug(fixtureRule)).toBe(expected)
  })
})

describe('formatFolio', () => {
  it('zero-pads to a width of two by default', () => {
    expect(formatFolio(1)).toBe('01')
  })

  it('honors a custom width', () => {
    expect(formatFolio(7, 3)).toBe('007')
  })
})

describe('inlineCode', () => {
  it.each([
    ['use `prose format`', 'use <code>prose format</code>'],
    ['<script>x</script>', '&lt;script&gt;x&lt;/script&gt;']
  ])('renders inline code and escapes raw markup in %j', (input, expected) => {
    expect(inlineCode(input)).toBe(expected)
  })
})

describe('requireString', () => {
  it('returns a non-empty string unchanged', () => {
    expect(requireString('align-equals', 'missing slug')).toBe('align-equals')
  })

  it.each([
    ['an empty string',      ''],
    ['a whitespace string',  '   '],
    ['a number',             7],
    ['a null',               null],
    ['an undefined',         undefined]
  ])('throws the message for %s', (_name, value) => {
    expect(() => requireString(value, 'missing slug')).toThrow('missing slug')
  })
})

describe('parseSvg', () => {
  it('exposes the viewBox and body of a parsed svg', () => {
    const parsed = parseSvg('<svg xmlns="x" viewBox="0 0 24 24"><path d="M0 0"/></svg>', 'icon.svg')
    expect(parsed.attribs.viewBox).toBe('0 0 24 24')
    expect(parsed.body).toContain('<path')
  })

  it('names the asset when the viewBox is absent', () => {
    expect(() => parseSvg('<svg><path/></svg>', 'icon.svg')).toThrow('icon.svg carries no viewBox')
  })
})

describe('stripSuffix', () => {
  it.each([
    ['rules/align.md', '.md', 'rules/align'],
    ['plain-text',     '.md', 'plain-text']
  ])('strips %j from %j only when present', (input, suffix, expected) => {
    expect(stripSuffix(input, suffix)).toBe(expected)
  })
})

describe('externalAttrs', () => {
  it.each([
    ['https://example.com', { rel: 'noopener', target: '_blank' }],
    ['http://example.com',  { rel: 'noopener', target: '_blank' }],
    ['/local/path',         {}],
    [undefined,             {}]
  ])('maps %s', (href, expected) => {
    expect(externalAttrs(href)).toEqual(expected)
  })
})

describe('lookup', () => {
  const registry = { alpha: 1, beta: 2 }

  it('returns the registered value', () => {
    expect(lookup(registry, 'alpha', 'Thing')).toBe(1)
  })

  it('throws with the sorted available keys', () => {
    expect(() => lookup(registry, 'gamma', 'Thing'))
      .toThrow('Thing "gamma" not registered. Available: alpha, beta')
  })
})

describe('railPaint', () => {
  it.each([
    [[],            'var(--vp-c-divider)'],
    [[null],        'var(--vp-c-divider)'],
    [['alignment'], 'var(--prose-family-alignment)']
  ])('paints a single or empty rail %j', (families, expected) => {
    expect(railPaint(families)).toBe(expected)
  })

  it('builds a gradient across multiple families', () => {
    expect(railPaint(['alignment', 'ordering'])).toBe(
      'linear-gradient(to bottom, var(--prose-family-alignment), var(--prose-family-ordering))'
    )
  })

  it('honors a custom direction', () => {
    expect(railPaint(['lint', 'docs'], 'to right')).toBe(
      'linear-gradient(to right, var(--prose-family-lint), var(--prose-family-docs))'
    )
  })

  const familyArb = fc.constantFrom('alignment', 'docs', 'formatting', 'layout', 'lint', 'ordering')

  test.prop([fc.array(familyArb, { minLength: 2, maxLength: 5 })])(
    'names every family token in a multi-family gradient',
    (families) => {
      const out = railPaint(families)
      for (const family of families) expect(out).toContain(`var(--prose-family-${family})`)
    }
  )
})

describe('withFallback', () => {
  it('resolves the function result on success', async () => {
    await expect(withFallback('demo', () => 42, 0)).resolves.toBe(42)
  })

  warnTest('resolves the fallback and warns on throw', async ({ warn }) => {
    await expect(withFallback('demo', () => { throw new Error('boom') }, 7)).resolves.toBe(7)
    expect(warn).toHaveBeenCalledOnce()
  })
})
