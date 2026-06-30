import { z } from 'astro/zod'

import { GLOSSARY_FAMILIES, PRIMITIVE_LAYERS, PRIMITIVE_STABILITIES } from '../shared/registries'

const DIRECTIVE_SCOPES = ['block', 'file', 'line'] as const
const PART_ROLES       = ['action', 'comment', 'namespace', 'payload'] as const

// The rule and primitive frontmatter the `docs` collection carries beyond
// Starlight's own fields, every field optional because one schema spans the
// rules, primitives, and prose pages alike, with the per-section requirements
// enforced by the cross-record integrity pass.
export const docsExtension = z.object({
  caption    : z.string().optional(),
  consumedBy : z.array(z.string()).optional(),
  consumes   : z.array(z.string()).optional(),
  layer      : z.enum(PRIMITIVE_LAYERS).optional(),
  related    : z.array(z.string()).optional(),
  stability  : z.enum(PRIMITIVE_STABILITIES).optional(),
  summary    : z.string().optional(),
  tagline    : z.string().optional()
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
  name : z.string(),
  role : z.string()
})

export const tokenIndex = z.array(z.object({
  blurb : z.string(),
  href  : z.string(),
  key   : z.string()
}))

export const exitCode = z.object({
  code    : z.number(),
  detail  : z.array(z.string()).nonempty(),
  label   : z.string(),
  summary : z.string()
})

export const directive = z.object({
  aliasOf   : z.string().optional(),
  effect    : z.string(),
  example   : z.string(),
  form      : z.string(),
  pairId    : z.string().optional(),
  pairRole  : z.enum(['closes', 'opens']).optional(),
  parts     : z.array(z.object({ role: z.enum(PART_ROLES), text: z.string() })).nonempty(),
  scope     : z.enum(DIRECTIVE_SCOPES),
  scopeNote : z.string().optional()
})

export const editorConfig = z.object({
  caption  : z.string(),
  code     : z.string(),
  language : z.string(),
  name     : z.string(),
  target   : z.string()
})

export const shellCompletion = z.object({
  caption  : z.string(),
  code     : z.string(),
  command  : z.string(),
  language : z.string(),
  mono     : z.string(),
  name     : z.string(),
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

export const fixture = z.object({
  canonical   : z.boolean().optional(),
  description : z.string().optional(),
  input       : z.string(),
  output      : z.string(),
  previewable : z.boolean().optional(),
  title       : z.string().optional(),
  findings    : z.array(z.object({
    code         : z.string(),
    end_location : findingLocation,
    fix          : z.object({
      applicability : z.string(),
      edits         : z.array(z.object({ before: z.string(), content: z.string() }))
    }).nullable(),
    location     : findingLocation,
    message      : z.string()
  }))
})

export const pipelineEntry = z.object({
  imperative : z.string(),
  position   : z.number(),
  slug       : z.string()
})

export const release = z.object({ version: z.string() })

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
