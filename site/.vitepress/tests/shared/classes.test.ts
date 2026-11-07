import fs   from 'node:fs'
import path from 'node:path'

import { babelParse, parse as parseSfc, walk } from '@vue/compiler-sfc'
import { parse }                               from 'postcss'

import { siteSheets, VITEPRESS_THEME } from '../support'
import * as tailwind                   from '../tailwind'
import * as vocabulary                 from '../vocabulary'

import type { Declaration, Root, Rule } from 'postcss'

type Statement  = ReturnType<typeof babelParse>['program']['body'][number]
type Expression = Extract<Statement, { type: 'ExpressionStatement' }>['expression']
type Variant    = NonNullable<ReturnType<typeof DESIGN.parseVariant>>

// Names one class an element can carry, beside the slot of its `class` and
// `:class` bindings that writes it and the alternative within that slot, so
// two classes in different alternatives of one ternary never meet. A class the
// root of a child component writes is marked inherited, since Vue adds the
// classes a parent passes to that root.
interface Written {
  alt        : number
  inherited ?: boolean
  name       : string
  slot       : number
}

interface Element {
  classes : Written[]
  line    : number
  linked ?: boolean
  tag    ?: string
}

// Names the variants whose arbitrary value is the attribute value they match.
const ATTRIBUTES = new Set(['aria', 'data'])
// Matches each class a selector or a selector string names.
const CLASS_NAME = /\.(-?[_a-zA-Z][\w-]*)/g
// Matches the `group` and `peer` classes, optionally named, that mark an
// element for the `group-*` and `peer-*` variants and generate no CSS.
const MARKER     = /^(?:group|peer)(?:\/[\w-]+)?$/
const SHEETS     = siteSheets()
const VOCAB      = vocabulary.siteVocabulary(
  SHEETS.find(({ file }) => file === vocabulary.TOKENS)!.root,
  SHEETS.map(({ root }) => root)
)
const DESIGN     = await tailwind.designSystem()
const SFCS       = tailwind.sourceFiles().filter(file => file.endsWith('.vue'))
const TEMPLATES  = [...SFCS, ...tailwind.sourceFiles().filter(file => file.endsWith('.tsx'))]
const COMPONENTS = new Map(TEMPLATES.map(file => [path.basename(file, path.extname(file)), file]))
// Names each breakpoint narrower than the widest the theme declares, each of
// which a template reads through its `max-*` variant.
const NARROW     = (() => {
  const theme  = vocabulary.read('.vitepress/theme/styles/tailwind/theme.css')
  const widths = [...theme.matchAll(/--breakpoint-([\w-]+)\s*:\s*(\d+)px/g)].map(([, name, px]) => ({ name, px: +px }))
  const widest = Math.max(...widths.map(({ px }) => px))
  return new Set(widths.filter(({ px }) => px < widest).map(({ name }) => name))
})()

// Names each class a stylesheet the site loads declares, its own sheets and the
// VitePress default theme's alike.
const DECLARED = new Set(
  [...SHEETS.map(({ root }) => root), ...vocabulary.vitepressSheets()].flatMap(classesOf)
)

// Names each word that follows a `.` in a quoted string of a script or a test,
// so every class a selector names passes as a hook, and so does a file
// extension or a domain label such as `json` or `org`.
const HOOKS = new Set(
  fs.globSync(['.vitepress/{kit,lib,tests,theme}/**/*.{ts,tsx,vue}', 'prototypes/**/*.{ts,tsx,vue}'], { cwd: vocabulary.SITE })
    .flatMap(file => [...vocabulary.read(file).matchAll(/(['"`])((?:(?!\1).)*)\1/g)])
    .flatMap(([, , text]) => [...text.matchAll(CLASS_NAME)].map(([, name]) => name))
)

// Returns every class name the selectors of a stylesheet name.
function classesOf(root: Root): string[] {
  const names = [] as string[]
  root.walkRules(rule => {
    for (const [, name] of rule.selector.matchAll(CLASS_NAME)) names.push(name)
  })
  return names
}

// Lists each element a template writes a class on, with the classes its static
// `class` attribute and its `:class` binding can give it.
function elementsOf(file: string): Element[] {
  if (file.endsWith('.tsx')) return jsxElementsOf(file)
  const ast = parseSfc(vocabulary.read(file)).descriptor.template?.ast
  const out = [] as Element[]
  const visit = (node: { children?: unknown[], loc: { start: { line: number } }, props?: unknown[], tag?: string }, within = false) => {
    const linked  = within || node.tag === 'a'
    const classes = [] as Written[]
    let slot = 1
    for (const prop of (node.props ?? []) as Record<string, any>[]) {
      if (prop.name === 'class' && prop.value) {
        classes.push(...words(prop.value.content).map(name => ({ alt: 0, name, slot: 0 })))
      } else if (prop.name === 'bind' && prop.arg?.content === 'class' && prop.exp) {
        const [statement] = babelParse(`(${prop.exp.content})`).program.body
        if (statement.type === 'ExpressionStatement') slot = bound(statement.expression, classes, slot)
      }
    }
    if (classes.length > 0) {
      const inherited = rootClassesOf(node.tag ?? '').map(name => ({ alt: 0, inherited: true, name, slot: 0 }))
      out.push({ classes: [...classes, ...inherited], line: node.loc.start.line, linked, tag: node.tag })
    }
    for (const child of node.children ?? []) visit(child as typeof node, linked)
  }
  if (ast) visit(ast as never)
  return out
}

// Lists each JSX element a kit file writes a class on, with the classes its
// `class` attribute can give it.
function jsxElementsOf(file: string): Element[] {
  const out = [] as Element[]
  walk(babelParse(vocabulary.read(file), { plugins: ['jsx', 'typescript'], sourceType: 'module' }) as never, {
    enter(raw: unknown) {
      const node = raw as Record<string, any>
      if (node.type !== 'JSXOpeningElement') return
      const classes = [] as Written[]
      for (const { name, value } of node.attributes as Record<string, any>[]) {
        if (name?.name === 'class' && value) bound(value.expression ?? value, classes, 1)
      }
      if (classes.length > 0) {
        const inherited = rootClassesOf(node.name.name ?? '').map(name => ({ alt: 0, inherited: true, name, slot: 0 }))
        out.push({ classes: [...classes, ...inherited], line: node.loc.start.line, linked: node.name.name === 'a', tag: node.name.name })
      }
    }
  })
  return out
}

// Returns the classes the static `class` attribute of a component's root
// element writes, where the component renders exactly one root element.
function rootClassesOf(tag: string): string[] {
  const file = COMPONENTS.get(tag)
  if (file?.endsWith('.tsx')) return jsxRootClassesOf(file)
  const roots = (file ? parseSfc(vocabulary.read(file)).descriptor.template?.ast?.children ?? [] : [])
    .filter(node => 'tag' in node) as Record<string, any>[]
  const attr  = roots.length === 1
    ? (roots[0].props as Record<string, any>[]).find(prop => prop.name === 'class' && prop.value)
    : undefined
  return attr ? words(attr.value.content) : []
}

// Returns the classes the static `class` attribute of a kit file's outermost
// JSX element writes, where the file writes exactly one such element.
function jsxRootClassesOf(file: string): string[] {
  const roots = [] as Record<string, any>[]
  walk(babelParse(vocabulary.read(file), { plugins: ['jsx', 'typescript'], sourceType: 'module' }) as never, {
    enter(raw: unknown) {
      const node = raw as Record<string, any>
      if (node.type !== 'JSXElement') return
      roots.push(node)
      this.skip()
    }
  })
  const attr = roots.length === 1
    ? (roots[0].openingElement.attributes as Record<string, any>[])
      .find(({ name, value }) => name?.name === 'class' && value?.type === 'StringLiteral')
    : undefined
  return attr ? words(attr.value.value) : []
}

// Adds the classes a `:class` expression can write, each string in a slot of its
// own unless a ternary holds it, in which case each branch is one alternative of
// a shared slot, the static `class` attribute holding slot 0. A class an
// expression computes at run time is left unread.
function bound(node: Expression, into: Written[], slot: number, alt = 0, shared?: number): number {
  const place = (name: string) => into.push({ alt, name, slot: shared ?? slot++ })
  switch (node.type) {
    case 'StringLiteral':
      for (const name of words(node.value)) place(name)
      break
    case 'TemplateLiteral':
      if (node.expressions.length === 0) for (const name of words(node.quasis[0].value.cooked ?? '')) place(name)
      break
    case 'ArrayExpression':
      for (const element of node.elements) {
        if (element && element.type !== 'SpreadElement') slot = bound(element, into, slot, alt, shared)
      }
      break
    case 'ObjectExpression':
      for (const property of node.properties) {
        if (property.type !== 'ObjectProperty') continue
        const key = property.key
        if (key.type === 'StringLiteral') for (const name of words(key.value)) place(name)
        if (key.type === 'Identifier' && !property.computed) place(key.name)
      }
      break
    case 'ConditionalExpression': {
      const own = shared ?? slot++
      bound(node.consequent, into, slot, alt * 2 + 1, own)
      bound(node.alternate, into, slot, alt * 2 + 2, own)
      break
    }
    case 'LogicalExpression':
      slot = bound(node.right, into, slot, alt, shared)
      break
  }
  return slot
}

// Names each class a VitePress template writes, through a static `class`
// attribute or as a key or a string its `:class` binding holds.
function vitepressClasses(): string[] {
  const keyed = (expression: string) => [...expression.matchAll(/([\w-]+)\s*:(?!:)|'([^']*)'/g)]
    .flatMap(([, key, text]) => (key ? [key] : words(text)))
  return fs.globSync(`${VITEPRESS_THEME}/**/*.vue`, { cwd: vocabulary.SITE })
    .flatMap(file => [...vocabulary.read(file).matchAll(/(?<!\w)(:?)class="([^"]+)"/g)])
    .flatMap(([, binding, names]) => (binding ? keyed(names) : words(names)))
}

const words = (text: string): string[] => text.split(/\s+/).filter(Boolean)

// Reports whether a variant writes an arbitrary value, leaving out the attribute
// value a `data-*` or `aria-*` variant matches.
function arbitraryVariant(variant: Variant): boolean {
  switch (variant.kind) {
    case 'arbitrary':  return true
    case 'compound':   return arbitraryVariant(variant.variant)
    case 'functional': return variant.value?.kind === 'arbitrary' && !ATTRIBUTES.has(variant.root)
    default:           return false
  }
}

// Reports whether a variant, or the variant a compound one wraps, applies above
// a breakpoint narrower than the widest.
function minWidthOfNarrow(variant: Variant): boolean {
  return variant.kind === 'compound' ? minWidthOfNarrow(variant.variant)
    : variant.kind === 'static' && NARROW.has(variant.root)
}

// Reports why a class fails, meaning an arbitrary value or variant, a value no
// token declares, a min-width variant of a breakpoint narrower than the widest,
// or a name neither a utility nor a stylesheet declares, and returns nothing for
// a class that passes. The `@property` rules Tailwind registers beside a utility
// hold its own initial values rather than the utility's, so the check reads the
// utility's rules alone. It passes the share a numeric modifier writes, and the
// lengths a static utility writes, since the stylesheet gate reads each
// `@utility` the site declares.
function problemOf(name: string): string | undefined {
  if (MARKER.test(name)) return undefined
  const [candidate] = DESIGN.parseCandidate(name)
  if (!candidate) return DECLARED.has(name) || HOOKS.has(name) ? undefined : 'unknown class'
  const arbitrary = candidate.kind === 'arbitrary'
    || ('value' in candidate && candidate.value?.kind === 'arbitrary')
    || ('modifier' in candidate && candidate.modifier?.kind === 'arbitrary')
    || candidate.variants.some(arbitraryVariant)
  if (arbitrary) return 'arbitrary value'
  if (candidate.variants.some(minWidthOfNarrow)) {
    return 'min-width variant of a narrow breakpoint'
  }
  const css = DESIGN.candidatesToCss([name])[0]
  if (!css) return 'unknown class'
  const root = parse(css)
  root.walkAtRules('property', rule => { rule.remove() })
  const share  = 'modifier' in candidate && candidate.modifier ? `${candidate.modifier.value}%` : undefined
  const strays = VOCAB.strays(root).filter(({ kind, value }) =>
    !(kind === 'share' && value === share) && !(kind === 'length' && candidate.kind === 'static'))
  return strays.length > 0 ? `writes ${strays.map(({ value }) => value).join(', ')}` : undefined
}

// Returns the variants a utility applies under and the properties it sets on
// the element carrying it, its `--tw-*` custom properties included. It leaves
// out each declaration that reads a `--tw-*` property, the hook another utility
// composes through (`transition-*` reading `--tw-duration`, which `duration-*`
// sets). It also leaves out each declaration a nested rule writes for a
// descendant, a sibling, or a pseudo-element.
function effectOf(name: string): { properties: Set<string>, variants: string } | undefined {
  const [candidate] = DESIGN.parseCandidate(name)
  const css         = candidate && DESIGN.candidatesToCss([name])[0]
  if (!css) return undefined
  const properties = new Set<string>()
  parse(css).walkDecls(decl => {
    if (!decl.value.startsWith('var(--tw-') && decl.parent?.type === 'rule' && !styledElsewhere(decl)) {
      properties.add(decl.prop)
    }
  })
  return { properties, variants: candidate.variants.map(variant => DESIGN.printVariant(variant)).join(':') }
}

// Reports whether a declaration sits in a nested rule whose subject, the last
// compound of its selector, is a box other than the element `&` names.
function styledElsewhere(decl: Declaration): boolean {
  for (let node = decl.parent; node?.type === 'rule'; node = node.parent) {
    const { selector } = node as Rule
    const subject      = selector.split(/[\s>+~]+(?![^(]*\))/).at(-1)!
    if (selector.includes('&') && (!subject.includes('&') || subject.includes('::'))) return true
  }
  return false
}

// Lists each pair of utilities an element can carry together that set one
// property under the same variants.
function conflictsOf({ classes }: Element): string[] {
  const found = [] as string[]
  for (const [i, a] of classes.entries()) {
    const first = effectOf(a.name)
    for (const b of classes.slice(i + 1)) {
      if ((a.slot === b.slot && a.alt !== b.alt) || (a.inherited && b.inherited)) continue
      const second = effectOf(b.name)
      if (!first || !second || first.variants !== second.variants) continue
      const shared = [...first.properties].filter(property => second.properties.has(property))
      if (shared.length > 0) found.push(`${a.name} and ${b.name} both set ${shared.join(', ')}`)
    }
  }
  return found
}

// Lists each class setting an `em`, `i`, `strong`, `b`, or `mark` outside a link
// in a color other than a text color or the aside ink.
function inlineHuesOf({ classes, linked, tag }: Element): string[] {
  if (linked || !vocabulary.INLINE_TEXT.has(tag ?? '')) return []
  return classes.map(({ name }) => name).filter(name => {
    let hue = false
    parse(DESIGN.candidatesToCss([name])[0] ?? '').walkDecls('color', decl => {
      hue ||= !vocabulary.TEXT_COLOR.test(decl.value)
    })
    return hue
  })
}

describe('the classes a template writes', () => {
  it('reads the classes a kit file writes on each element', () => {
    const kicker = TEMPLATES.find(file => file.endsWith('kit/chips/Kicker.tsx'))!
    expect(elementsOf(kicker).flatMap(({ classes }) => classes.map(({ name }) => name))).toContain('kicker')
  })

  it.each(TEMPLATES)('%s writes only declared, token-backed, unconflicted classes', file => {
    const problems = elementsOf(file).flatMap(element => [
      ...element.classes.filter(({ inherited }) => !inherited).flatMap(({ name }) => {
        const problem = problemOf(name)
        return problem ? [`${element.line}: ${name}, ${problem}`] : []
      }),
      ...conflictsOf(element).map(conflict => `${element.line}: ${conflict}`)
    ])
    expect(problems).toStrictEqual([])
  })

  it.each(TEMPLATES)('%s sets each em, i, strong, b, and mark outside a link in a text color or the aside ink', file => {
    expect(elementsOf(file).flatMap(inlineHuesOf)).toStrictEqual([])
  })

  it.each([
    { element: { tag: 'em' },                 hues: ['text-brand-1'] },
    { element: { tag: 'strong' },             hues: ['text-brand-1'] },
    { element: { linked: true, tag: 'em' },   hues: [] },
    { element: { tag: 'span' },               hues: [] }
  ])('reports a hue on $element.tag as $hues', ({ element, hues }) => {
    const classes = [{ alt: 0, name: 'text-brand-1', slot: 0 }, { alt: 0, name: 'text-text-1', slot: 0 }]
    expect(inlineHuesOf({ ...element, classes, line: 1 })).toStrictEqual(hues)
  })

  it('generates no utility from an arbitrary value or a stray literal', () => {
    const blocked  = tailwind.blockedCandidates()
    const problems = [...new Set(tailwind.scannedCandidates())]
      .filter(name => !blocked.has(name) && DESIGN.candidatesToCss([name])[0])
      .flatMap(name => {
        const problem = problemOf(name)
        return problem ? [`${name}, ${problem}`] : []
      })
    expect(problems).toStrictEqual([])
  })

  it.each([
    { name: 'p-[13px]',                 problem: 'arbitrary value' },
    { name: '[&>*]:p-14',               problem: 'arbitrary value' },
    { name: 'bg-ube/[0.3]',             problem: 'arbitrary value' },
    { name: 'outline-offset-7',         problem: 'writes 7px' },
    { name: 'no-such-class',            problem: 'unknown class' },
    { name: 'mobile:p-14',              problem: 'min-width variant of a narrow breakpoint' },
    { name: 'not-tablet:p-14',          problem: 'min-width variant of a narrow breakpoint' },
    { name: 'p-14',                     problem: undefined },
    { name: 'max-mobile:p-14',          problem: undefined },
    { name: 'wide:p-14',                problem: undefined },
    { name: 'bg-ube/55',                problem: undefined },
    { name: 'sr-only',                  problem: undefined },
    { name: 'data-[edge=end]:p-14',     problem: undefined },
    { name: 'aria-[current=page]:p-14', problem: undefined },
    { name: 'group/link',               problem: undefined }
  ])('reports $name as $problem', ({ name, problem }) => {
    expect(problemOf(name)).toBe(problem)
  })

  it.each([
    {
      case      : 'two static classes setting one property',
      classes   : [{ alt: 0, name: 'text-ube', slot: 0 }, { alt: 0, name: 'text-text-1', slot: 0 }],
      conflicts : ['text-ube and text-text-1 both set color']
    },
    {
      case      : 'a shorthand beside a longhand Tailwind sorts after it',
      classes   : [{ alt: 0, name: 'p-14', slot: 0 }, { alt: 0, name: 'pt-2', slot: 0 }],
      conflicts : []
    },
    {
      case      : 'the two branches of one ternary',
      classes   : [{ alt: 1, name: 'text-ube', slot: 1 }, { alt: 2, name: 'text-text-1', slot: 1 }],
      conflicts : []
    },
    {
      case      : 'a utility styling a descendant beside one setting that property on the element',
      classes   : [{ alt: 0, name: 'fixture-card-body-inner', slot: 0 }, { alt: 0, name: 'm-14', slot: 0 }],
      conflicts : []
    },
    {
      case      : 'a class a parent passes against the root class of its child',
      classes   : [{ alt: 0, name: 'tracking-display-wide', slot: 0 }, { alt: 0, inherited: true, name: 'kicker', slot: 0 }],
      conflicts : ['tracking-display-wide and kicker both set letter-spacing']
    },
    {
      case      : 'two root classes of one child',
      classes   : [{ alt: 0, inherited: true, name: 'text-ube', slot: 0 }, { alt: 0, inherited: true, name: 'text-text-1', slot: 0 }],
      conflicts : []
    }
  ])('finds the conflicts of $case', ({ classes, conflicts }) => {
    expect(conflictsOf({ classes, line: 1 })).toStrictEqual(conflicts)
  })

  it('marks the root classes of a child component as inherited', () => {
    const inherited = elementsOf('.vitepress/theme/components/layout/NotFound.tsx')
      .flatMap(({ classes }) => classes.filter(written => written.inherited))
    expect(inherited).toStrictEqual([{ alt: 0, inherited: true, name: 'kicker', slot: 0 }])
  })

  it('reads the root classes of a component that renders one root element', () => {
    expect(rootClassesOf('Kicker')).toStrictEqual(['kicker'])
    expect(rootClassesOf('Chip')).toStrictEqual([])
  })
})

describe('the Tailwind theme', () => {
  const theme = vocabulary.read('.vitepress/theme/styles/tailwind/theme.css')

  it('builds a utility value from every token the multiplier and the bare modifier leave', () => {
    const tokens = [...vocabulary.read(vocabulary.TOKENS).matchAll(
      /^\s*(--prose-(?:border|leading|radius|size|space|text|tint|tracking|width)(?:-[\w-]+)?)\s*:/gm
    )].map(([, token]) => token)
      .filter(token => !/^--prose-(?:(?:size|space|width)-\d+(?:-5)?|tint-\d+)$/.test(token))
    expect(tokens.filter(token => !theme.includes(`var(${token})`))).toStrictEqual([])
  })

  it('writes each multiple of the spacing token through the multiplier', () => {
    expect(DESIGN.candidatesToCss(['w-28', 'p-0.5'])).toStrictEqual([
      '.w-28 {\n  width: calc(var(--prose-space-1) * 28);\n}\n',
      '.p-0\\.5 {\n  padding: calc(var(--prose-space-1) * 0.5);\n}\n'
    ])
  })

  it('applies dark under the VitePress class', () => {
    expect(DESIGN.candidatesToCss(['dark:text-ube'])[0]).toContain(':where(.dark, .dark *)')
  })

  it('generates no utility named for a class a VitePress template writes or binds', () => {
    const blocked   = tailwind.blockedCandidates()
    const scanned   = new Set(tailwind.scannedCandidates())
    const generated = [...new Set(vitepressClasses())]
      .filter(name => scanned.has(name) && !blocked.has(name) && DESIGN.candidatesToCss([name])[0])
    expect(generated).toStrictEqual([])
  })
})
