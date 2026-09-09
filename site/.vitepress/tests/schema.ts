import { repoRoot }                from '../lib/shared/paths'
import { proseSchema, ruleDefsOf } from '../lib/shared/rule-schema'

// Holds the `prose schema` output and the rule table its readers index, which
// the suites reading either one share rather than each spawning the binary.
export const SCHEMA    = proseSchema(repoRoot(import.meta.url))
export const RULE_DEFS = ruleDefsOf(SCHEMA)
