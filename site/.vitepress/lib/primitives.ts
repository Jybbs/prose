export const PRIMITIVES = {
  'source'           : 'Source',
  'pipeline'         : 'Pipeline',
  'binding-analysis' : 'BindingAnalysis',
  'suppression-map'  : 'SuppressionMap',
  'rule-id'          : 'RuleId'
} as const satisfies Record<string, string>

export type PrimitiveSlug = keyof typeof PRIMITIVES
