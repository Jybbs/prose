import { firstFit }        from '../shared/first-fit'
import type { RuleFamily } from '../shared/registries'

interface OrderedRule {
  after  : readonly string[]
  family : RuleFamily | null
  slug   : string
}

// How one rule stands to a chosen rule in the pipeline's declared order.
export type OrderRelation = 'earlier' | 'later' | 'self' | undefined

export interface OrderMark {
  family : RuleFamily | null
  slug   : string
  x      : number
  y      : number
}

// The drawing of one rule's place in the pipeline, a line spanning every pass
// with the rule's own pass marked, each rule it runs after as a badge above
// the line, and each rule running after it as a badge below.
export interface OrderLine {
  earlier : readonly OrderMark[]
  height  : number
  here    : number
  later   : readonly OrderMark[]
  lineY   : number
}

export const BADGE = 12

const GAP = 2

// Names `rule` as the chosen rule itself, a rule the chosen one declares it runs
// after, a rule declaring it runs after the chosen one, or none of these.
export function orderRelation(chosen: OrderedRule, rule: OrderedRule): OrderRelation {
  return rule === chosen                  ? 'self'
       : chosen.after.includes(rule.slug) ? 'earlier'
       : rule.after.includes(chosen.slug) ? 'later'
       : undefined
}

// Seats each badge in the first row where it clears the badge last seated
// there, opening a new row where no row has room, and returns each row.
function seatRows(xs: readonly number[]): number[] {
  const ends: number[] = []
  return xs.map(x => firstFit(ends, x, end => x - end >= BADGE))
}

// Lays out the order line for `slug` across `width`, setting a badge one row
// further from the line where it would overlap the badge before it, with as
// many rows a side as the most crowded rule in `rules` needs, so the height
// stays the same for every rule.
export function orderLine(rules: readonly OrderedRule[], slug: string, width: number): OrderLine {
  const step = (width - BADGE) / Math.max(rules.length - 1, 1)
  const xOf  = (index: number): number => BADGE / 2 + index * step

  const picked = (chosen: OrderedRule, relation: OrderRelation): number[] =>
    rules.flatMap((rule, index) => orderRelation(chosen, rule) === relation ? [index] : [])
  const rowsOf = (chosen: OrderedRule, relation: OrderRelation): number[] =>
    seatRows(picked(chosen, relation).map(xOf))

  const rows  = Math.max(1, ...rules.flatMap(rule =>
    [...rowsOf(rule, 'earlier'), ...rowsOf(rule, 'later')].map(row => row + 1)))
  const lineY = rows * (BADGE + GAP)
  const at    = rules.findIndex(rule => rule.slug === slug)

  const place = (relation: OrderRelation, toward: -1 | 1): OrderMark[] => {
    if (at === -1) return []
    const indexes = picked(rules[at], relation)
    const seats   = seatRows(indexes.map(xOf))
    return indexes.map((index, nth) => {
      const offset = GAP + seats[nth] * (BADGE + GAP)
      return {
        family : rules[index].family,
        slug   : rules[index].slug,
        x      : xOf(index),
        y      : toward < 0 ? lineY - offset - BADGE : lineY + offset
      }
    })
  }

  return {
    earlier : place('earlier', -1),
    height  : 2 * lineY,
    here    : xOf(at),
    later   : place('later', 1),
    lineY   : lineY
  }
}
