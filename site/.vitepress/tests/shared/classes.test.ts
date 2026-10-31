import fs   from 'node:fs'
import path from 'node:path'

import { babelParse, parse as parseSfc } from '@vue/compiler-sfc'
import { parse }                         from 'postcss'

import { siteSheets }  from '../support'
import * as tailwind   from '../tailwind'
import * as vocabulary from '../vocabulary'

import type { Declaration, Root, Rule } from 'postcss'

type Statement  = ReturnType<typeof babelParse>['program']['body'][number]
type Expression = Extract<Statement, { type: 'ExpressionStatement' }>['expression']

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
}

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
const TEMPLATES  = tailwind.sourceFiles().filter(file => file.endsWith('.vue'))
const COMPONENTS = new Map(TEMPLATES.map(file => [path.basename(file, '.vue'), file]))

// Names each class a stylesheet the site loads declares, its own sheets and the
// VitePress default theme's alike.
const DECLARED = new Set(
  [...SHEETS.map(({ root }) => root), ...vocabulary.vitepressSheets()].flatMap(classesOf)
)

// Names each class a script or a test reads as a selector, which a template
// keeps as a hook whether or not a stylesheet styles it.
const HOOKS = new Set(
  fs.globSync(['.vitepress/{lib,tests,theme}/**/*.{ts,vue}', 'prototypes/**/*.{ts,vue}'], { cwd: vocabulary.SITE })
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
  const ast = parseSfc(vocabulary.read(file)).descriptor.template?.ast
  const out = [] as Element[]
  const visit = (node: { children?: unknown[], loc: { start: { line: number } }, props?: unknown[], tag?: string }) => {
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
      out.push({ classes: [...classes, ...inherited], line: node.loc.start.line })
    }
    for (const child of node.children ?? []) visit(child as typeof node)
  }
  if (ast) visit(ast as never)
  return out
}

// Returns the classes the static `class` attribute of a component's root
// element writes, where the component renders exactly one root element.
function rootClassesOf(tag: string): string[] {
  const file  = COMPONENTS.get(tag)
  const roots = (file ? parseSfc(vocabulary.read(file)).descriptor.template?.ast?.children ?? [] : [])
    .filter(node => 'tag' in node) as Record<string, any>[]
  const attr  = roots.length === 1
    ? (roots[0].props as Record<string, any>[]).find(prop => prop.name === 'class' && prop.value)
    : undefined
  return attr ? words(attr.value.content) : []
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
  return fs.globSync(`${vocabulary.VITEPRESS_THEME}/**/*.vue`, { cwd: vocabulary.SITE })
    .flatMap(file => [...vocabulary.read(file).matchAll(/(?<!\w)(:?)class="([^"]+)"/g)])
    .flatMap(([, binding, names]) => (binding ? keyed(names) : words(names)))
}

const words = (text: string): string[] => text.split(/\s+/).filter(Boolean)

// Reports why a class fails, meaning an arbitrary value or variant, a value no
// token declares, or a name neither a utility nor a stylesheet declares, and
// returns nothing for a class that passes. The `@property` rules Tailwind
// registers beside a utility hold its own initial values rather than the
// utility's, so the check reads the utility's rules alone.
function problemOf(name: string): string | undefined {
  if (MARKER.test(name)) return undefined
  const [candidate] = DESIGN.parseCandidate(name)
  if (!candidate) return DECLARED.has(name) || HOOKS.has(name) ? undefined : 'unknown class'
  const arbitrary = candidate.kind === 'arbitrary'
    || ('value' in candidate && candidate.value?.kind === 'arbitrary')
    || ('modifier' in candidate && candidate.modifier?.kind === 'arbitrary')
    || candidate.variants.some(variant =>
      variant.kind === 'arbitrary' || ('value' in variant && variant.value?.kind === 'arbitrary'))
  if (arbitrary) return 'arbitrary value'
  const css = DESIGN.candidatesToCss([name])[0]
  if (!css) return 'unknown class'
  const root = parse(css)
  root.walkAtRules('property', rule => { rule.remove() })
  const strays = VOCAB.strays(root)
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

describe('the classes a template writes', () => {
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

  it.each([
    { name: 'p-[13px]',         problem: 'arbitrary value' },
    { name: '[&>*]:p-14',       problem: 'arbitrary value' },
    { name: 'bg-ube/[0.3]',     problem: 'arbitrary value' },
    { name: 'outline-offset-7', problem: 'writes 7px' },
    { name: 'no-such-class',    problem: 'unknown class' },
    { name: 'p-14',             problem: undefined },
    { name: 'group/link',       problem: undefined }
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
      case      : 'a utility styling only a descendant beside one sizing the element',
      classes   : [{ alt: 0, name: 'tool-ring', slot: 0 }, { alt: 0, name: 'size-22', slot: 0 }],
      conflicts : []
    },
    {
      case      : 'a class a parent passes against the root class of its child',
      classes   : [{ alt: 0, name: 'tracking-display-wide', slot: 0 }, { alt: 0, inherited: true, name: 'kicker', slot: 0 }],
      conflicts : ['tracking-display-wide and kicker both set letter-spacing']
    }
  ])('finds the conflicts of $case', ({ classes, conflicts }) => {
    expect(conflictsOf({ classes, line: 1 })).toStrictEqual(conflicts)
  })

  it('reads the root classes of a component that renders one root element', () => {
    expect(rootClassesOf('Kicker')).toStrictEqual(['kicker'])
    expect(rootClassesOf('Chip')).toStrictEqual([])
  })
})

describe('the Tailwind theme', () => {
  const theme = vocabulary.read('.vitepress/theme/styles/tailwind/theme.css')

  it('builds a utility value from every length and share token', () => {
    const tokens = [...vocabulary.read(vocabulary.TOKENS).matchAll(
      /^\s*(--prose-(?:border|leading|radius|size|space|text|tint|tracking|width)(?:-[\w-]+)?)\s*:/gm
    )].map(([, token]) => token)
    expect(tokens.filter(token => !theme.includes(`var(${token})`))).toStrictEqual([])
  })

  it('declares a variant for every breakpoint tokens.css declares', () => {
    const variants = new Set([...vocabulary.read(tailwind.ENTRY).matchAll(
      /@custom-variant ([\w-]+) \(@media \(--prose-bp-\1\)\)/g
    )].map(([, name]) => name))
    const breakpoints = [...vocabulary.read(vocabulary.TOKENS).matchAll(/@custom-media --prose-bp-([\w-]+)/g)]
      .map(([, name]) => name)
    expect(breakpoints.filter(name => !variants.has(name))).toStrictEqual([])
  })

  it('generates no utility named for a class a VitePress template writes or binds', () => {
    const blocked   = tailwind.blockedCandidates()
    const scanned   = new Set(tailwind.scannedCandidates())
    const generated = [...new Set(vitepressClasses())]
      .filter(name => scanned.has(name) && !blocked.has(name) && DESIGN.candidatesToCss([name])[0])
    expect(generated).toStrictEqual([])
  })
})
