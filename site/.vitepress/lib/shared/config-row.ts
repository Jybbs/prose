import { inlineNodes }                   from '../markdown/inline-nodes.ts'
import type { InlineNode, InlineParser } from '../markdown/inline-nodes.ts'
import { typeOf }                        from './rule-schema.ts'
import type { SchemaProp }               from './rule-schema.ts'

// The record `ConfigRowTable` renders for one configuration key.
export interface ConfigRow {
  default      : string
  key          : string
  meaningNodes : InlineNode[]
  typeNodes    : InlineNode[]
}

export function configRow(
  md    : InlineParser,
  key   : string,
  prop  : SchemaProp,
  value : unknown
): ConfigRow {
  return {
    default      : value === null ? 'unset' : JSON.stringify(value),
    key          : key,
    meaningNodes : inlineNodes(md, prop.description ?? ''),
    typeNodes    : inlineNodes(md, typeOf(prop))
  }
}
