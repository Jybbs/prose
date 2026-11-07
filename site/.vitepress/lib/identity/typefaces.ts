// Lists each typographic role the typefaces specimen sets on its small page,
// as the element and class the page's own stylesheets set it through and the
// words it is set in.
export const TYPE_ROLES = [
  { id: 'h1',      label: 'Title',         tag: 'h1',   class: undefined,       sample: 'Visual Identity' },
  { id: 'h2',      label: 'Section head',  tag: 'h2',   class: undefined,       sample: 'Typefaces' },
  { id: 'h3',      label: 'Entry',         tag: 'h3',   class: undefined,       sample: 'Columns That Line Up' },
  { id: 'body',    label: 'Body',          tag: 'p',    class: undefined,       sample: 'Code is read far more often than it is written,' },
  { id: 'aside',   label: 'Aside',         tag: 'em',   class: undefined,       sample: 'so the page sets it for the reader.' },
  { id: 'code',    label: 'Code',          tag: 'code', class: undefined,       sample: 'prose check .' },
  { id: 'kicker',  label: 'Kicker',        tag: 'span', class: 'kicker',        sample: 'Reference · Part I' },
  { id: 'th',      label: 'Table head',    tag: 'th',   class: undefined,       sample: 'Default' },
  { id: 'callout', label: 'Callout title', tag: 'span', class: 'callout-title', sample: 'Format would change' },
  { id: 'folio',   label: 'Folio',         tag: 'span', class: 'folio',         sample: 'iv' }
] as const

export type TypeRoleId = (typeof TYPE_ROLES)[number]['id']

// Lists the properties a reading takes its tokens from.
const TYPE_PROPERTIES = ['font-family', 'font-size', 'font-style', 'font-weight', 'letter-spacing', 'line-height'] as const

export interface TypeReading {
  face     : string
  family   : string
  selector : string
  size     : number
  style    : string
  tokens   : readonly string[]
  tracking : string
  weight   : string
}

// Lists the style rules in `rules` that apply to `element`, entering every
// grouping rule apart from a media rule whose query fails in the window.
function matching(rules: CSSRuleList, element: Element, win: Window): CSSStyleRule[] {
  return [...rules].flatMap(rule => {
    if (rule instanceof CSSStyleRule) return element.matches(rule.selectorText) ? [rule] : []
    if (rule instanceof CSSMediaRule && !win.matchMedia(rule.conditionText).matches) return []
    return 'cssRules' in rule ? matching(rule.cssRules as CSSRuleList, element, win) : []
  })
}

// Returns the rule among `rules` whose declaration of `property` sets the
// value the element computes, found by setting each candidate on a probe
// beside the element, the last such candidate winning.
function winning(rules: readonly CSSStyleRule[], property: string, element: HTMLElement, win: Window): CSSStyleRule | undefined {
  const computed = win.getComputedStyle(element).getPropertyValue(property)
  const probe    = element.ownerDocument.createElement(element.tagName)
  element.after(probe)
  const found = rules.filter(rule => rule.style.getPropertyValue(property)).findLast(rule => {
    probe.style.setProperty(property, rule.style.getPropertyValue(property))
    return win.getComputedStyle(probe).getPropertyValue(property) === computed
  })
  probe.remove()
  return found
}

// Reads the face and its family stack, the size, weight, style, and tracking
// `element` computes, beside each custom property the declarations setting
// them read and the selector of the rule setting its family.
export function readType(element: HTMLElement): TypeReading {
  const win    = element.ownerDocument.defaultView!
  const style  = win.getComputedStyle(element)
  const rules  = [...element.ownerDocument.styleSheets].flatMap(sheet => matching(sheet.cssRules, element, win))
  const won    = Object.fromEntries(TYPE_PROPERTIES.map(property => [property, winning(rules, property, element, win)]))
  const tokens = TYPE_PROPERTIES.flatMap(property =>
    [...(won[property]?.style.getPropertyValue(property) ?? '').matchAll(/var\((--[\w-]+)/g)].map(([, name]) => name!))
  return {
    face     : style.fontFamily.split(',')[0]!.trim().replaceAll(/^["']|["']$/g, ''),
    family   : style.fontFamily,
    selector : won['font-family']?.selectorText ?? element.tagName.toLowerCase(),
    size     : parseFloat(style.fontSize),
    style    : style.fontStyle,
    tokens   : [...new Set(tokens)],
    tracking : style.letterSpacing,
    weight   : style.fontWeight
  }
}

