import { readHelp }                from '../lib/reference/cli-help'
import { repoRoot }                from '../lib/shared/paths'
import { proseSchema, ruleDefsOf } from '../lib/shared/rule-schema'

// Holds the `prose schema` output, the rule table its readers index, and every
// `--help` page the site reads, each read once and shared by every suite.
export const HELP      = readHelp(repoRoot(import.meta.url))
export const SCHEMA    = proseSchema(repoRoot(import.meta.url))
export const RULE_DEFS = ruleDefsOf(SCHEMA)
