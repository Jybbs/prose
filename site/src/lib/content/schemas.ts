import { z } from 'astro/zod'

import {
  FAMILY_WARMTHS, GLOSSARY_FAMILIES, PRIMITIVE_LAYERS, PRIMITIVE_STABILITIES
} from '../shared/registries'
import { required } from '../shared/required'
import { SCOPES }   from '../suppression/scope-meta'

const DIRECTIVE_FORM = /^(#) ([a-z]+:) ([a-z]+)(\[.*\])?$/
const PART_ROLES     = ['action', 'comment', 'namespace', 'payload'] as const

interface DirectivePart {
  role : (typeof PART_ROLES)[number]
  text : string
}

// Tokenizes a directive form into its anatomy parts, the leading `#` as the
// comment, the `word:` head as the namespace, the verb as the action, and a
// trailing bracket run as the payload.
function directiveParts(form: string): DirectivePart[] {
  const match = required(DIRECTIVE_FORM.exec(form), `directive form "${form}" does not tokenize`)
  const parts: DirectivePart[] = [
    { role: 'comment',   text: match[1] },
    { role: 'namespace', text: match[2] },
    { role: 'action',    text: match[3] }
  ]
  if (match[4] !== undefined) parts.push({ role: 'payload', text: match[4] })
  return parts
}

// The rule, family-index, and primitive frontmatter the `docs` collection
// carries beyond Starlight's own fields, every field optional because one
// schema spans the rules, primitives, and prose pages alike, with the
// per-section requirements enforced by the cross-record integrity pass.
export const docsExtension = z.object({
  badge      : z.string().optional(),
  caption    : z.string().optional(),
  consumedBy : z.array(z.string()).optional(),
  consumes   : z.array(z.string()).optional(),
  layer      : z.enum(PRIMITIVE_LAYERS).optional(),
  related    : z.array(z.string()).optional(),
  stability  : z.enum(PRIMITIVE_STABILITIES).optional(),
  summary    : z.string().optional(),
  tagline    : z.string().optional(),
  warmth     : z.enum(FAMILY_WARMTHS).optional()
})

export type DocsFrontmatter = z.infer<typeof docsExtension>

export const glossary = z.object({
  aliases    : z.array(z.string()).optional(),
  definition : z.string(),
  families   : z.array(z.enum(GLOSSARY_FAMILIES)).nonempty(),
  href       : z.string().optional(),
  rule       : z.string().optional()
})

export const tool = z.object({
  href : z.string(),
  icon : z.string(),
  name : z.string()
})

export const tokenIndex = z.object({
  label   : z.string(),
  entries : z.array(z.object({
    blurb : z.string(),
    href  : z.string(),
    key   : z.string()
  })).nonempty().optional()
})

export const exitCode = z.object({
  code    : z.number(),
  detail  : z.array(z.string()).nonempty(),
  label   : z.string(),
  summary : z.string()
})

export const directive = z
  .object({
    aliasOf  : z.string().optional(),
    effect   : z.string(),
    example  : z.string(),
    form     : z.string().regex(DIRECTIVE_FORM),
    pairId   : z.string().optional(),
    pairRole : z.enum(['closes', 'opens']).optional(),
    scope    : z.enum(SCOPES)
  })
  .transform(data => ({ ...data, parts: directiveParts(data.form) }))

export const editorConfig = z.object({
  caption  : z.string(),
  code     : z.string(),
  language : z.string(),
  target   : z.string()
})

export const shellCompletion = z.object({
  caption  : z.string(),
  code     : z.string(),
  language : z.string(),
  note     : z.string(),
  target   : z.string()
})

export const ruleConfigPreset = z.object({
  rows: z.array(z.object({
    default : z.string(),
    key     : z.string(),
    meaning : z.string(),
    type    : z.string()
  })).nonempty()
})

export const landingSurface = z.object({ body: z.string() })

export const landingStep = z.object({
  body     : z.string(),
  code     : z.string(),
  language : z.string(),
  title    : z.string()
})

export const composition = z
  .object({ harness: z.object({ rules: z.array(z.string()).nonempty() }) })
  .transform(({ harness }) => ({ rules: harness.rules }))

const findingLocation = z.object({ column: z.number(), row: z.number() })

const finding = z.object({
  code         : z.string(),
  end_location : findingLocation,
  location     : findingLocation,
  message      : z.string(),
  fix          : z.object({
    applicability : z.string(),
    edits         : z.array(z.object({ before: z.string(), content: z.string() }))
  }).nullable()
})

export type LintFinding = z.infer<typeof finding>

export const fixture = z.object({
  canonical   : z.boolean().optional(),
  description : z.string().optional(),
  findings    : z.array(finding),
  input       : z.string(),
  output      : z.string(),
  previewable : z.boolean().optional(),
  steps       : z.array(z.unknown()).optional(),
  title       : z.string().optional()
})

export const pipelineEntry = z.object({
  imperative : z.string(),
  position   : z.number(),
  slug       : z.string()
})

export type PipelineEntry = z.infer<typeof pipelineEntry>

export const release = z.object({ gitSha: z.string(), version: z.string() })

export const stars = z.object({ stars: z.string() })

export const pypiRelease = z.object({
  date      : z.string(),
  month     : z.string(),
  url       : z.string(),
  version   : z.string(),
  year      : z.string(),
  yearShort : z.string()
})

export const typingDemo = z.object({
  prelude          : z.string(),
  pythonStateSteps : z.array(z.unknown()),
  resetRows        : z.array(z.object({ anchor: z.string(), end: z.string(), prelude: z.string() })),
  entries          : z.array(z.object({
    anchor : z.string(),
    from   : z.string(),
    kind   : z.literal('edit'),
    slug   : z.string(),
    tail   : z.string().optional(),
    to     : z.string()
  }))
})

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const FIXTURE = {
    description : 'a demo pair',
    input       : 'x=1',
    output      : 'x = 1',
    previewable : true,
    findings    : [
      { code: 'E1', end_location: { column: 2, row: 1 }, location: { column: 1, row: 1 }, message: 'm', fix: null },
      { code: 'E2', end_location: { column: 2, row: 1 }, location: { column: 1, row: 1 }, message: 'm',
        fix: { applicability: 'safe', edits: [{ before: 'x=1', content: 'x = 1' }] } }
    ]
  }

  const TYPING_DEMO = {
    prelude          : 'p',
    pythonStateSteps : [{}],
    resetRows        : [{ anchor: 'a', end: 'e', prelude: 'p' }],
    entries          : [
      { anchor: 'a', from: 'f', kind: 'edit', slug: 's', to: 't' },
      { anchor: 'a', from: 'f', kind: 'edit', slug: 's', tail: 'z', to: 't' }
    ]
  }

  describe('schemas accept valid frontmatter', () => {
    test.each([
      { name: 'docsExtension, every field absent',      schema: docsExtension,    input: {} },
      { name: 'docsExtension, every field present',     schema: docsExtension,    input: {
        badge   : 'b', caption: 'c', consumedBy: ['a'], consumes: ['b'], layer: 'base',
        related : ['r'], stability: 'public', summary: 's', tagline: 't', warmth: 'warm'
      } },
      { name: 'glossary with aliases and rule',         schema: glossary,         input: {
        aliases: ['a'], definition: 'd', families: ['cli', 'engine'], href: '/x', rule: 'r'
      } },
      { name: 'tool',                                   schema: tool,             input: { href: '/h', icon: 'i', name: 'n' } },
      { name: 'tokenIndex without entries',             schema: tokenIndex,       input: { label: 'l' } },
      { name: 'tokenIndex with entries',                schema: tokenIndex,       input: { label: 'l', entries: [{ blurb: 'b', href: '/h', key: 'k' }] } },
      { name: 'exitCode',                               schema: exitCode,         input: { code: 0, detail: ['d'], label: 'l', summary: 's' } },
      { name: 'editorConfig',                           schema: editorConfig,     input: { caption: 'c', code: 'x', language: 'py', target: 't' } },
      { name: 'shellCompletion',                        schema: shellCompletion,  input: { caption: 'c', code: 'x', language: 'bash', note: 'n', target: 't' } },
      { name: 'ruleConfigPreset',                       schema: ruleConfigPreset, input: { rows: [{ default: 'd', key: 'k', meaning: 'm', type: 't' }] } },
      { name: 'landingSurface',                         schema: landingSurface,   input: { body: 'b' } },
      { name: 'landingStep',                            schema: landingStep,      input: { body: 'b', code: 'c', language: 'py', title: 't' } },
      { name: 'fixture with a null and a present fix',  schema: fixture,          input: FIXTURE },
      { name: 'pipelineEntry',                          schema: pipelineEntry,    input: { imperative: 'do', position: 1, slug: 's' } },
      { name: 'release',                                schema: release,          input: { gitSha: 'abc', version: '0.1.0' } },
      { name: 'stars',                                  schema: stars,            input: { stars: '100' } },
      { name: 'pypiRelease',                            schema: pypiRelease,      input: {
        date: '2026-01-01', month: 'Jan', url: '/u', version: '0.1.0', year: '2026', yearShort: '26'
      } },
      { name: 'typingDemo',                             schema: typingDemo,       input: TYPING_DEMO }
    ])('$name', ({ schema, input }) => {
      expect(schema.safeParse(input).success).toBe(true)
    })
  })

  describe('schemas reject invalid frontmatter', () => {
    test.each([
      { name: 'docsExtension with an unknown layer',   schema: docsExtension,    input: { layer: 'bogus' } },
      { name: 'docsExtension with an unknown warmth',  schema: docsExtension,    input: { warmth: 'hot' } },
      { name: 'glossary with empty families',          schema: glossary,         input: { definition: 'd', families: [] } },
      { name: 'glossary with an unknown family',       schema: glossary,         input: { definition: 'd', families: ['bogus'] } },
      { name: 'glossary missing its definition',       schema: glossary,         input: { families: ['cli'] } },
      { name: 'tool missing its href',                 schema: tool,             input: { icon: 'i', name: 'n' } },
      { name: 'tokenIndex with empty entries',         schema: tokenIndex,       input: { label: 'l', entries: [] } },
      { name: 'exitCode with empty detail',            schema: exitCode,         input: { code: 0, detail: [], label: 'l', summary: 's' } },
      { name: 'exitCode with a non-numeric code',      schema: exitCode,         input: { code: 'x', detail: ['d'], label: 'l', summary: 's' } },
      { name: 'editorConfig missing its code',         schema: editorConfig,     input: { caption: 'c', language: 'py', target: 't' } },
      { name: 'shellCompletion missing its note',      schema: shellCompletion,  input: { caption: 'c', code: 'x', language: 'bash', target: 't' } },
      { name: 'ruleConfigPreset with empty rows',      schema: ruleConfigPreset, input: { rows: [] } },
      { name: 'landingSurface missing its body',       schema: landingSurface,   input: {} },
      { name: 'landingStep missing its title',         schema: landingStep,      input: { body: 'b', code: 'c', language: 'py' } },
      { name: 'fixture missing its input',             schema: fixture,          input: { findings: [], output: 'o' } },
      { name: 'fixture with a malformed finding',      schema: fixture,          input: {
        findings: [{ code: 'E', end_location: { column: 1, row: 1 }, location: { column: 1, row: 1 }, fix: null }],
        input   : 'i', output: 'o'
      } },
      { name: 'pipelineEntry with a non-numeric position', schema: pipelineEntry, input: { imperative: 'd', position: 'x', slug: 's' } },
      { name: 'release missing its version',           schema: release,          input: { gitSha: 'abc' } },
      { name: 'stars with a non-string count',         schema: stars,            input: { stars: 100 } },
      { name: 'typingDemo with an unknown edit kind',  schema: typingDemo,       input: {
        ...TYPING_DEMO, entries: [{ anchor: 'a', from: 'f', kind: 'append', slug: 's', to: 't' }]
      } }
    ])('$name', ({ schema, input }) => {
      expect(schema.safeParse(input).success).toBe(false)
    })
  })

  const directiveInput = (form: string) => ({ effect: 'e', example: 'x', form, scope: 'file' as const })

  describe('directive tokenizes its form', () => {
    test.each([
      { name: 'a bare directive', form: '# fmt: off', parts: [
        { role: 'comment',   text: '#' },
        { role: 'namespace', text: 'fmt:' },
        { role: 'action',    text: 'off' }
      ] },
      { name: 'a bracket payload', form: '# prose: skip[align-equals]', parts: [
        { role: 'comment',   text: '#' },
        { role: 'namespace', text: 'prose:' },
        { role: 'action',    text: 'skip' },
        { role: 'payload',   text: '[align-equals]' }
      ] }
    ])('$name', ({ form, parts }) => {
      expect(directive.parse(directiveInput(form)).parts).toEqual(parts)
    })

    test.each([
      { name: 'a capitalized namespace', form: '# Fmt: off' },
      { name: 'a missing comment hash',  form: 'fmt: off' },
      { name: 'a missing colon',         form: '# fmt off' }
    ])('rejects $name', ({ form }) => {
      expect(directive.safeParse(directiveInput(form)).success).toBe(false)
    })
  })

  describe('composition lifts the harness rules', () => {
    test('valid input', () => {
      expect(composition.parse({ harness: { rules: ['align-equals', 'align-colons'] } }))
        .toEqual({ rules: ['align-equals', 'align-colons'] })
    })

    test('rejects an empty rule set', () => {
      expect(composition.safeParse({ harness: { rules: [] } }).success).toBe(false)
    })
  })
}
