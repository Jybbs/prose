export const PRIMITIVES = {
  'binding-analysis' : 'BindingAnalysis',
  'pipeline'         : 'Pipeline',
  'rule-id'          : 'RuleId',
  'source'           : 'Source',
  'suppression-map'  : 'SuppressionMap'
} as const satisfies Record<string, string>

export type PrimitiveSlug = keyof typeof PRIMITIVES
