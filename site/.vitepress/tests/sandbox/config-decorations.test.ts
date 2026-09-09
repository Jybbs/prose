import type { RuleControl } from '../../lib/sandbox/config-schema.data'
import {
  configDecorations,
  flaggedRows,
  noticeKey,
  rankedFacets,
  unknownKeyFindings
} from '../../lib/sandbox/config-decorations'

const notice = (key: string) => `warning: unknown key \`${key}\` in [tool.prose]`

describe('unknownKeyFindings', () => {
  it('spans the key a root-level notice names', () => {
    const toml = 'code-line-length = 100\nno-such-key = 1\n'

    expect(unknownKeyFindings([notice('no-such-key')], toml)).toStrictEqual([{
      code         : 'config-key',
      end_location : { column: 12, row: 2 },
      location     : { column: 1,  row: 2 },
      message      : notice('no-such-key')
    }])
  })

  it('resolves a dotted path against the table its line sits under', () => {
    const toml = '[rules.align-equals]\nno-such-facet = true\n'
    const [finding] = unknownKeyFindings([notice('rules.align-equals.no-such-facet')], toml)

    expect(finding?.location).toStrictEqual({ column: 1, row: 2 })
    expect(finding?.end_location).toStrictEqual({ column: 14, row: 2 })
  })

  it('reads a key written in quotes', () => {
    const toml = '[rules]\n"no-such-rule" = false\n'
    const [finding] = unknownKeyFindings([notice('rules.no-such-rule')], toml)

    expect(finding?.end_location).toStrictEqual({ column: 15, row: 2 })
  })

  it('reads a key written dotted on its own line', () => {
    const toml = 'rules.align-equals.no-such-facet = true\n'

    expect(unknownKeyFindings([notice('rules.align-equals.no-such-facet')], toml)).toHaveLength(1)
  })

  it('keeps an indented key at the column it sits in', () => {
    const toml = '[rules]\n  no-such-rule = false\n'
    const [finding] = unknownKeyFindings([notice('rules.no-such-rule')], toml)

    expect(finding?.location).toStrictEqual({ column: 3, row: 2 })
  })

  it('leaves a key the source does not carry to the message list', () => {
    expect(unknownKeyFindings([notice('absent')], 'code-line-length = 100\n')).toStrictEqual([])
  })

  it('leaves a notice of another shape alone', () => {
    expect(unknownKeyFindings(['something else entirely'], 'no-such-key = 1\n')).toStrictEqual([])
  })

  it('reads past an array-of-tables header', () => {
    const toml = '[[overrides]]\npaths = ["x"]\nno-such-key = 1\n'
    const [finding] = unknownKeyFindings([notice('overrides.no-such-key')], toml)

    expect(finding?.location).toStrictEqual({ column: 1, row: 3 })
  })
})

describe('flaggedRows', () => {
  it('names each row carrying an unrecognized key once', () => {
    const toml = 'no-such-key = 1\ncode-line-length = 100\n[rules.align-equals]\nno-such-facet = true\n'
    const notices = [notice('no-such-key'), notice('rules.align-equals.no-such-facet')]

    expect(flaggedRows(notices, toml)).toStrictEqual([1, 4])
  })
})

describe('configDecorations', () => {
  it('carries the notice as the hover message under the lint flag class', () => {
    const toml = 'no-such-key = 1\n'

    expect(configDecorations([notice('no-such-key')], toml)).toStrictEqual([{
      end        : { character: 11, line: 0 },
      properties : {
        class          : 'lint-flag underline-draw',
        'data-message' : notice('no-such-key'),
        'data-rule'    : 'config-key'
      },
      start      : { character: 0, line: 0 }
    }])
  })
})

const facet = (key: string) =>
  ({ default: true, hintHtml: '', key, kind: 'bool' as const, label: key })

const ALIGN_EQUALS: RuleControl = {
  facets : [facet('enabled'), facet('max-shift')],
  family : 'alignment',
  slug   : 'align-equals'
}

describe('rankedFacets', () => {
  it('leads with the facet closest to what the reader wrote', () => {
    expect(rankedFacets('rules.align-equals.max-shfit', [ALIGN_EQUALS]).map(f => f.key))
      .toStrictEqual(['max-shift', 'enabled'])
  })

  it('reorders when the reader was closer to the other facet', () => {
    expect(rankedFacets('rules.align-equals.enabld', [ALIGN_EQUALS]).map(f => f.key))
      .toStrictEqual(['enabled', 'max-shift'])
  })

  it('offers nothing for a root key, which names no rule', () => {
    expect(rankedFacets('no-such-key', [ALIGN_EQUALS])).toStrictEqual([])
  })

  it('offers nothing for a rule the sandbox does not carry', () => {
    expect(rankedFacets('rules.no-such-rule.enabled', [ALIGN_EQUALS])).toStrictEqual([])
  })
})

describe('noticeKey', () => {
  it('reads the path out of a notice and rejects another shape', () => {
    expect(noticeKey(notice('rules.align-equals.max-shfit'))).toBe('rules.align-equals.max-shfit')
    expect(noticeKey('something else')).toBeNull()
  })
})
