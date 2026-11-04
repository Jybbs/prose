import fs   from 'node:fs'
import path from 'node:path'

import { siteDir }                             from '../../lib/shared/paths'
import { configKeySources, facetDestinations } from '../../lib/tokens/config-keys'
import { destinations, headingsOf }            from '../../lib/tokens/destinations'
import { SCHEMA }                              from '../schema'
import { supportTest }                         from '../support'

describe('headingsOf', () => {
  it('keys each heading by its id, as plain words, marking one set wholly as code', () => {
    expect(headingsOf('# Page\n\n## `prose format`\n\n### Hit-Miss **Telemetry**\n\n## The [Cache](/a)')).toStrictEqual(new Map([
      ['prose-format',       { code: true,  text: 'prose format' }],
      ['hit-miss-telemetry', { code: false, text: 'Hit-Miss Telemetry' }],
      ['the-cache',          { code: false, text: 'The Cache' }]
    ]))
  })

  it('skips a heading-shaped line inside a code fence', () => {
    expect(headingsOf('```bash\n## not a heading\n```\n## Real').size).toBe(1)
  })
})

describe('destinations', () => {
  supportTest('names the page, the heading, or the anchor a link leads to', ({ tmpDir }) => {
    fs.mkdirSync(path.join(tmpDir, 'reference'))
    fs.writeFileSync(path.join(tmpDir, 'reference', 'cli.md'), '# CLI\n\n## `prose check`\n')
    fs.writeFileSync(path.join(tmpDir, 'reference', 'index.md'), '# Reference\n')
    const destinationOf = destinations(tmpDir, new Map([['/reference/cli#facet', { code: true, text: 'align-colons' }]]))
    expect(destinationOf('/reference/cli')).toStrictEqual({ code: false, part: null, title: 'CLI' })
    expect(destinationOf('/reference/cli#prose-check')).toStrictEqual({ code: true, part: 'prose check', title: 'CLI' })
    expect(destinationOf('/reference/cli#facet')).toStrictEqual({ code: true, part: 'align-colons', title: 'CLI' })
    expect(destinationOf('/reference/')).toStrictEqual({ code: false, part: null, title: 'Reference' })
  })

  supportTest('throws on a page with no H1 and on an anchor nothing carries', ({ tmpDir }) => {
    fs.writeFileSync(path.join(tmpDir, 'bare.md'), 'No heading.\n')
    fs.writeFileSync(path.join(tmpDir, 'page.md'), '# Page\n')
    const destinationOf = destinations(tmpDir, new Map())
    expect(() => destinationOf('/bare')).toThrow(/\/bare has no H1/)
    expect(() => destinationOf('/page#missing')).toThrow(/names no heading or anchor/)
  })
})

describe('facetDestinations', () => {
  it('resolves every configuration key a token names, through a heading or a facet anchor', () => {
    const destinationOf = destinations(siteDir(import.meta.url), facetDestinations(SCHEMA))
    for (const { href } of configKeySources(SCHEMA)) expect(() => destinationOf(href)).not.toThrow()
  })

  it('names a hoisted facet by its scope and a rule\'s own facet by the rule, set as code', () => {
    const named = facetDestinations(SCHEMA)
    expect(named.get('/reference/configuration#every-rule-enabled')).toStrictEqual({ code: false, text: 'every rule' })
    expect(named.get('/reference/configuration#reflow-calls-max-args')).toStrictEqual({ code: true, text: 'reflow-calls' })
  })
})
