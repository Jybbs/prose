import fs                from 'node:fs'
import path              from 'node:path'
import { pathToFileURL } from 'node:url'

import { ExpressiveCodeEngine }               from '@expressive-code/core'
import { toHtml }                             from 'hast-util-to-html'
import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { beforeEach, describe, expect, test, vi }        from 'vitest'
import type { Data }                          from 'mdast'
import type { JSXNode }                       from 'satori/jsx'
import type { AstroComponentFactory }         from 'astro/runtime/server/index.js'
import { visitParents }                       from 'unist-util-visit-parents'

import type {
  DocsVocab,
  GlossaryRef,
  PrimitiveRef,
  RuleRef
} from '../src/lib/content/discovery/docs-vocab'
import { conditionalLoad }                    from '../src/lib/content/loaders/conditional'
import { pipelineLoader, releaseLoader }      from '../src/lib/content/loaders/crate'
import { docsLoaderWithIntegrity }            from '../src/lib/content/loaders/docs-loader'
import { fixturesLoader }                     from '../src/lib/content/loaders/fixtures'
import { pypiReleasesLoader }                 from '../src/lib/content/loaders/pypi'
import { replaceStore }                       from '../src/lib/content/loaders/store'
import type { StoreEntry }                    from '../src/lib/content/loaders/store'
import { starsLoader }                        from '../src/lib/content/loaders/stars'
import type { LintFinding }                   from '../src/lib/content/schemas'
import {
  fixture as fixtureSchema, pipelineEntry, pypiRelease, release, stars, typingDemo
} from '../src/lib/content/schemas'
import { SOURCE }                             from '../src/lib/landing/typing-demo'
import { remarkBodyLink }                     from '../src/lib/markdown/body-link'
import { remarkGlossary }                     from '../src/lib/markdown/glossary-linker'
import { pluginLintFlag }                     from '../src/lib/markdown/lint-flag'
import { pushClassName }                      from '../src/lib/markdown/mdast-node'
import { remarkProseMark }                    from '../src/lib/markdown/prose-mark'
import { remarkRuleLinks }                    from '../src/lib/markdown/rule-links'
import { loadBrandAssets }                    from '../src/lib/og/assets'
import { landingSvg }                         from '../src/lib/og/landing'
import { enumerateCards }                     from '../src/lib/og/pages'
import type { OgPage }                        from '../src/lib/og/pages'
import { cardShell, dataPanel, el, leftRail, monoLabel, toSvg } from '../src/lib/og/parts'
import { pageSvg }                            from '../src/lib/og/template'
import { axeViolations }                      from './common/a11y'
import { cases }                              from './common/fixtures'
import type { Case }                          from './common/fixtures'
import { formatMarkup, normalize }            from './common/format'
import { makeContext }                        from './common/loader'
import type { Entry, FakeContext, Schema }    from './common/loader'
import { renderTransform }                    from './common/mdast'
import type { Transform }                     from './common/mdast'

const state = vi.hoisted(() => ({ store: {} as Record<string, { data: unknown, id: string }[]> }))
const H     = vi.hoisted(() => ({ cargoToml: { value: '' }, exec: vi.fn(), fetchRaw: vi.fn(), innerLoad: vi.fn() }))

vi.mock('astro:content',              async () => (await import('./common/support')).astroContent(state.store))
vi.mock('ofetch',                     () => ({ ofetch: { raw: H.fetchRaw } }))
vi.mock('node:child_process',         () => ({ execFileSync: H.exec }))
vi.mock('@astrojs/starlight/loaders', () => ({ docsLoader: () => ({ load: H.innerLoad, name: 'starlight-docs' }) }))

vi.mock('node:fs', async orig => {
  const actual     = await orig<typeof import('node:fs')>()
  const existsSync = (file: string) =>
    (typeof file === 'string' && file.endsWith('/prose')) || actual.existsSync(file)
  return { ...actual, default: { ...actual, existsSync }, existsSync }
})

vi.mock('node:fs/promises', async orig => {
  const actual = await orig<typeof import('node:fs/promises')>()
  const readFile = (file: string, encoding?: unknown): Promise<unknown> =>
    typeof file === 'string' && file.endsWith('Cargo.toml')
      ? Promise.resolve(H.cargoToml.value)
      : actual.readFile(file as never, encoding as never)
  return { ...actual, default: { ...actual, readFile } }
})

vi.mock('@astrojs/starlight/components/Footer.astro', async () => {
  const { createComponent, render } = await import('astro/runtime/server/index.js')
  return { default: createComponent(() => render`<div data-starlight-footer></div>`) }
})
vi.mock('@astrojs/starlight/components/SocialIcons.astro', async () => {
  const { createComponent, render } = await import('astro/runtime/server/index.js')
  return { default: createComponent(() => render`<span data-starlight-social></span>`) }
})
vi.mock('@astrojs/starlight/components/Head.astro', async () => {
  const { createComponent, render } = await import('astro/runtime/server/index.js')
  return {
    default: createComponent((_result: unknown, _props: unknown, slots: Record<string, () => unknown>) =>
      render`<starlight-head>${slots.default?.()}</starlight-head>`)
  }
})

type Meta   = Record<string, unknown>
type Handler = (c: Case) => Promise<void>

const snapshot = async (c: Case, file: string, text: string, format: 'markup' | 'raw'): Promise<void> => {
  const body = format === 'markup' ? await formatMarkup(text) : normalize(text)
  await expect(body).toMatchFileSnapshot(path.join(c.dir, file))
}

const seedStore = (store: Record<string, { data: unknown, id: string }[]>): void => {
  for (const key of Object.keys(state.store)) delete state.store[key]
  Object.assign(state.store, store)
}

const mapBy = <T>(rows: unknown, key: string): Map<string, T> =>
  new Map(((rows as Record<string, unknown>[]) ?? []).map(row => {
    const { [key]: id, ...rest } = row
    return [id as string, rest as T]
  }))

const vocabFrom = (meta: Meta): DocsVocab => ({
  glossary   : mapBy<GlossaryRef>(meta.glossary, 'term'),
  primitives : mapBy<PrimitiveRef>(meta.primitives, 'slug'),
  rules      : mapBy<RuleRef>(meta.rules, 'slug')
})

const seededPush = (meta: Meta): Transform => tree => {
  visitParents(tree, 'link', node => {
    if (meta.seed) node.data = meta.seed as Data
    pushClassName(node, 'body-link')
  })
}

const transformFor: Record<string, (meta: Meta) => Transform> = {
  'body-link'      : ()   => remarkBodyLink(),
  'glossary-linker': meta => remarkGlossary(vocabFrom(meta)),
  'mdast-node'     : meta => seededPush(meta),
  'prose-mark'     : ()   => remarkProseMark(),
  'rule-links'     : meta => remarkRuleLinks(vocabFrom(meta))
}

const lintFindings = (meta: Meta): Map<string, LintFinding[]> =>
  new Map(Object.entries((meta.findings ?? {}) as Record<string, LintFinding[]>)
    .map(([id, list]) => [id, list.map(finding => ({ ...finding, fix: finding.fix ?? null }))]))

const renderLintFence = async (meta: Meta, code: string): Promise<string> => {
  const engine = new ExpressiveCodeEngine({
    plugins: [pluginLintFlag(lintFindings(meta), mapBy<RuleRef>(meta.rules, 'slug'))]
  })
  const { renderedGroupAst } = await engine.render({
    code,
    language : (meta.language as string) ?? 'python',
    meta     : (meta.meta as string) ?? ''
  })
  return toHtml(renderedGroupAst)
}

const renderMarkdown = async (domain: string, meta: Meta, input: string): Promise<string> => {
  const produce = (): Promise<string> | string => domain === 'lint-flag'
    ? renderLintFence(meta, input)
    : renderTransform(transformFor[domain](meta), input)
  if (!meta.throws) return produce()
  try {
    await produce()
  } catch (error) {
    return (error as Error).message
  }
  throw new Error(`${domain} fixture expected a throw`)
}

const markdown: Handler = async c => {
  const input  = fs.readFileSync(path.join(c.dir, 'input.md'), 'utf8')
  const output = await renderMarkdown(c.subject, c.meta, input)
  await snapshot(c, 'input.md.snap', output, c.meta.throws ? 'raw' : 'markup')
}

const markdownEntries = Object.fromEntries([...Object.keys(transformFor), 'lint-flag'].map(d => [d, markdown]))

const reviveNulls = (value: unknown): unknown => {
  if (value === '@null') return null
  if (Array.isArray(value)) return value.map(reviveNulls)
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, reviveNulls(entry)]))
  }
  return value
}

const propsFrom = (meta: Meta): Record<string, unknown> => {
  const props = reviveNulls(meta.props ?? {}) as Record<string, unknown>
  for (const key of (meta.setProps as string[] | undefined) ?? []) props[key] = new Set(props[key] as unknown[])
  return props
}

const components = import.meta.glob<{ default: AstroComponentFactory }>('../src/components/**/*.astro')

const component: Handler = async c => {
  vi.resetModules()
  seedStore((c.meta.store as Record<string, { data: unknown, id: string }[]>) ?? {})
  const container = await AstroContainer.create()
  const { default: Component } = await components[`../src/components/${c.meta.component as string}`]()
  const options = { props: propsFrom(c.meta), slots: (c.meta.slots as Record<string, string>) ?? {} }

  if (c.meta.error !== undefined) {
    await expect(container.renderToString(Component, options)).rejects.toThrow(new RegExp(c.meta.error as string))
    return
  }

  const html = await container.renderToString(Component, options)
  await snapshot(c, 'output.html.snap', html, 'markup')
  const ignore = (c.meta.axeIgnore as string[] | undefined) ?? []
  expect((await axeViolations(html)).filter(id => !ignore.includes(id))).toEqual([])
}

const BRAND = loadBrandAssets()

const versionOf = (meta: Meta): string => (meta.version as string | undefined) ?? '0.1.0'

const enumerateJson = async (meta: Meta): Promise<string> => {
  seedStore({
    docs     : (meta.docs as { data: unknown, id: string }[] | undefined) ?? [],
    pipeline : (meta.pipeline as { data: unknown, id: string }[] | undefined) ?? []
  })
  return JSON.stringify(await enumerateCards(), null, 2)
}

const buildPart = (meta: Meta): JSXNode => {
  if (meta.part === 'leftRail') return leftRail(meta.color as string)
  if (meta.part === 'cardShell') {
    const kids = (meta.children as string[]).map(child => el('div', { children: child }))
    return cardShell(...kids)
  }
  if (meta.part === 'dataPanel') {
    const rows = meta.rows as ReadonlyArray<readonly [string, string]>
    return dataPanel(meta.accent as string, meta.alpha as string, rows, versionOf(meta))
  }
  if (meta.part === 'monoLabel') {
    const style = monoLabel(meta.color as string, meta.size as number, meta.track as string | undefined)
    return el('div', { children: meta.text as string, style })
  }
  return el('div', { style: { display: 'flex' } }, el('span', { children: meta.text as string }))
}

const ogRender: Record<string, (meta: Meta) => Promise<string>> = {
  enumerate : enumerateJson,
  landing   : meta => landingSvg(BRAND, versionOf(meta)),
  page      : meta => pageSvg(meta.page as OgPage, BRAND, versionOf(meta)),
  parts     : meta => toSvg(buildPart(meta), BRAND.fonts)
}

const og: Handler = async c => {
  const output = await ogRender[c.subject](c.meta)
  if (c.subject === 'enumerate') await snapshot(c, 'output.json.snap', output, 'raw')
  else                           await snapshot(c, 'output.svg.snap', output, 'markup')
}

const ogEntries = Object.fromEntries(Object.keys(ogRender).map(d => [d, og]))

const asMiddleware = (fn: unknown) => fn as (c: unknown, next: () => Promise<void>) => Promise<void>

const discoveryRunners: Record<string, (dir: string, options: Meta) => Promise<unknown>> = {
  'category-meta': async () =>
    (await import('../src/lib/content/discovery/family-meta')).CATEGORY_META,

  'head-middleware': async (_dir, options) => {
    const { onRequest } = await import('../src/lib/head/middleware')
    const head    = (options.head as unknown[]) ?? []
    const context = {
      locals : { starlightRoute: { entry: { data: options.data ?? {}, id: options.id }, head } },
      site   : options.site === undefined ? undefined : new URL(options.site as string)
    }
    await asMiddleware(onRequest)(context, async () => {})
    return head
  },

  'nav-middleware': async (_dir, options) => {
    const { onRequest } = await import('../src/lib/nav/middleware')
    const context = { locals: { starlightRoute: { sidebar: options.sidebar } } }
    await asMiddleware(onRequest)(context, async () => {})
    return options.sidebar
  },

  'page-timestamps': async (_dir, options) => {
    const { buildContentTimestamps } = await import('../src/lib/config/page-timestamps')
    if (options.gitFails === true) H.exec.mockImplementation(() => { throw new Error('not a repo') })
    else H.exec.mockReturnValue(options.log as string)
    const warn   = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const stamps = buildContentTimestamps(new URL('file:///repo/site/'))
    const warned = warn.mock.calls.length
    warn.mockRestore()
    return { timestamps: Object.fromEntries(stamps), warned }
  },

  'discover-docs-vocab': async dir => {
    const { discoverDocsVocab } = await import('../src/lib/content/discovery/docs-vocab')
    const vocab                 = discoverDocsVocab(pathToFileURL(`${dir}${path.sep}`))
    return {
      glossary   : Object.fromEntries(vocab.glossary),
      primitives : Object.fromEntries(vocab.primitives),
      rules      : Object.fromEntries(vocab.rules)
    }
  },

  'discovered-primitives': async () =>
    (await import('../src/lib/content/discovery/primitives')).discoveredPrimitives(),

  'discovered-rules': async () =>
    (await import('../src/lib/content/discovery/rules')).discoveredRules(),

  'family-badges': async () =>
    Object.fromEntries(await (await import('../src/lib/content/discovery/rules')).familyBadges()),

  'family-meta': async (_dir, options) =>
    (await (await import('../src/lib/content/discovery/family-meta')).familyMeta())(options.family as string),

  'glossary-folio-entries': async () =>
    (await import('../src/lib/glossary/entries')).glossaryFolioEntries(),

  'is-index': async (_dir, options) =>
    (await import('../src/lib/content/discovery/page')).isIndex(options.file as string),

  'layer-numeral': async () =>
    (await import('../src/lib/content/discovery/primitives')).LAYER_NUMERAL,

  'page-files': async dir => {
    const { pageFiles } = await import('../src/lib/content/discovery/page')
    return [...pageFiles(path.join(dir, 'pages'))].map(page => page.slug).sort()
  },

  'rule-by-slug': async (_dir, options) =>
    (await (await import('../src/lib/content/discovery/rules')).ruleBySlug(options.slug as string)) ?? null,

  'rule-or-throw': async (_dir, options) =>
    (await import('../src/lib/content/discovery/rules'))
      .ruleOrThrow(options.slug as string, options.consumer as string),

  'slug-of': async (_dir, options) =>
    (await import('../src/lib/content/discovery/page')).slugOf(options.file as string),

  'subdirectories': async dir =>
    (await import('../src/lib/content/discovery/page')).subdirectories(path.join(dir, 'pages')),

  'tool-namer': async (_dir, options) => {
    const name = await (await import('../src/lib/content/discovery/tool-names')).toolNamer(options.noun as string)
    return Object.fromEntries((options.ids as string[]).map(id => [id, name(id)]))
  }
}

const discovery: Handler = async c => {
  const { options: raw, ...store } = c.meta
  const options                    = (raw ?? {}) as Meta
  seedStore(store as Record<string, { data: unknown, id: string }[]>)
  vi.doMock('../src/lib/markdown/render', () => ({ renderInline: (md: string) => Promise.resolve(`<em>${md}</em>`) }))
  const run = discoveryRunners[c.subject](c.dir, options)
  if (typeof options.throws === 'string') await expect(run).rejects.toThrow(options.throws)
  else await snapshot(c, 'output.snap', JSON.stringify(await run, null, 2), 'raw')
}

const discoveryEntries = Object.fromEntries(Object.keys(discoveryRunners).map(d => [d, discovery]))

interface FetchMeta {
  data   ?: unknown
  etag   ?: string | null
  fail   ?: boolean
  ok     ?: boolean
  status ?: number
}

const SCHEMAS: Record<string, Schema> = { fixture: fixtureSchema, pipelineEntry, pypiRelease, release, stars, typingDemo }

const CONDITIONAL_SOURCE = {
  etagKey   : 'k:etag',
  fallback  : [{ data: { v: 'fallback' }, id: 'row' }],
  headers   : { Accept: 'application/json' },
  label     : 'demo',
  toEntries : (payload: unknown) => [{ data: { v: (payload as { v: string }).v }, id: 'row' }],
  url       : 'https://example.test/data'
}

const asEntries = (rows: unknown): Entry[] => (rows as Entry[] | undefined) ?? []
const schemaFor = (meta: Meta): Schema | undefined => typeof meta.schema === 'string' ? SCHEMAS[meta.schema] : undefined
const values    = (fake: FakeContext): Entry[] => [...fake.store.values()]

const fetchResponse = (fetch: FetchMeta): unknown => ({
  _data   : fetch.data,
  headers : { get: (name: string) => (name === 'etag' ? (fetch.etag ?? null) : null) },
  ok      : fetch.ok ?? ((fetch.status ?? 200) < 400),
  status  : fetch.status ?? 200
})

const configureFetch = (meta: Meta): void => {
  if (meta.offline === true) { vi.stubEnv('PROSE_OFFLINE_DOCS', '1'); return }
  const fetch = meta.fetch as FetchMeta | undefined
  if (fetch === undefined) return
  if (fetch.fail === true) H.fetchRaw.mockRejectedValueOnce(new Error('net'))
  else H.fetchRaw.mockResolvedValueOnce(fetchResponse(fetch) as never)
}

const loaderRunners: Record<string, (c: Case) => Promise<unknown>> = {
  conditional: async ({ meta }) => {
    configureFetch(meta)
    const fake = makeContext({
      meta  : meta.metaEtag !== undefined ? [['k:etag', meta.metaEtag as string]] : undefined,
      store : meta.warm === true ? [{ data: { v: 'cached' }, id: 'row' }] : undefined
    })
    await conditionalLoad(fake.ctx, CONDITIONAL_SOURCE)
    return {
      entries  : values(fake),
      meta     : Object.fromEntries(fake.meta),
      request  : H.fetchRaw.mock.calls[0],
      warnings : fake.warn.mock.calls.map(call => call[0])
    }
  },

  'docs-loader': async ({ meta }) => {
    const loader = docsLoaderWithIntegrity()
    const fake   = makeContext({ store: asEntries(meta.store) })
    try {
      await loader.load(fake.ctx)
    } catch (error) {
      return { error: (error as Error).message }
    }
    return { entries: values(fake), innerLoadCalls: H.innerLoad.mock.calls.length, name: loader.name }
  },

  'fixtures-loader': async ({ dir, meta }) => {
    const root = pathToFileURL(path.join(dir, 'site') + path.sep).href
    const fake = makeContext({ root, schema: schemaFor(meta) })
    await fixturesLoader().load(fake.ctx)
    return { entries: values(fake) }
  },

  pipeline: async ({ meta }) => {
    H.exec.mockReturnValue(meta.exec as string)
    const fake = makeContext({ schema: schemaFor(meta) })
    await pipelineLoader().load(fake.ctx)
    return { entries: values(fake), execArgs: H.exec.mock.calls }
  },

  pypi: async ({ meta }) => {
    configureFetch(meta)
    const fake = makeContext({ schema: schemaFor(meta) })
    await pypiReleasesLoader().load(fake.ctx)
    return { entries: values(fake), fetchCalls: H.fetchRaw.mock.calls.length }
  },

  release: async ({ meta }) => {
    H.cargoToml.value = meta.cargo as string
    if (meta.gitFails === true) H.exec.mockImplementation(() => { throw new Error('no git') })
    else H.exec.mockReturnValue(meta.git as string)
    const fake = makeContext({ schema: schemaFor(meta) })
    try {
      await releaseLoader().load(fake.ctx)
    } catch (error) {
      return { error: (error as Error).message }
    }
    return { entries: values(fake) }
  },

  stars: async ({ meta }) => {
    configureFetch(meta)
    const fake = makeContext({ schema: schemaFor(meta) })
    await starsLoader().load(fake.ctx)
    return { entries: values(fake), fetchCalls: H.fetchRaw.mock.calls.length }
  },

  store: async ({ meta }) => {
    const fake = makeContext({ schema: schemaFor(meta), store: asEntries(meta.initial) })
    try {
      await replaceStore(fake.ctx, meta.entries as StoreEntry[])
    } catch (error) {
      return { error: (error as Error).message }
    }
    return { entries: values(fake) }
  },

  'typing-demo-loader': async ({ meta }) => {
    vi.doMock('../src/lib/markdown/magic-move', () => ({
      precompileMagicMove: (states: readonly string[]) => states.map((code, step) => ({ chars: code.length, step }))
    }))
    const { typingDemoLoader } = await import('../src/lib/content/loaders/typing-demo')
    H.exec.mockReturnValue(SOURCE)
    const fake = makeContext({ schema: schemaFor(meta) })
    await typingDemoLoader().load(fake.ctx)
    const execCalls = H.exec.mock.calls.map(([, args, opts]) => ({
      hasCwd : (opts as { cwd?: string }).cwd !== undefined,
      select : (args as string[])[3]
    }))
    return { entries: values(fake), execCalls }
  }
}

const loader: Handler = async c => {
  const result = await loaderRunners[c.subject](c)
  await snapshot(c, 'output.snap', JSON.stringify(result, null, 2), 'raw')
}

const loaderEntries = Object.fromEntries(Object.keys(loaderRunners).map(d => [d, loader]))

const registry: Record<string, Handler> = {
  ...markdownEntries,
  ...ogEntries,
  ...discoveryEntries,
  ...loaderEntries
}

const dispatch = (c: Case): Promise<void> =>
  'component' in c.meta ? component(c) : registry[c.subject](c)

beforeEach(() => {
  H.exec.mockReset()
  H.fetchRaw.mockReset()
  H.innerLoad.mockReset()
  H.cargoToml.value = ''
  vi.unstubAllEnvs()
  vi.doUnmock('../src/lib/markdown/render')
  vi.doUnmock('../src/lib/markdown/magic-move')
  vi.resetModules()
  for (const key of Object.keys(state.store)) delete state.store[key]
})

const tree = new Map<string, Map<string, Case[]>>()
for (const c of cases()) {
  const subjects = tree.get(c.domain) ?? new Map<string, Case[]>()
  const group    = subjects.get(c.subject) ?? []
  group.push(c)
  subjects.set(c.subject, group)
  tree.set(c.domain, subjects)
}

for (const [domain, subjects] of tree) {
  describe(domain, () => {
    for (const [subject, group] of subjects) {
      describe(subject, () => {
        for (const c of group) test(c.name, () => dispatch(c), c.meta.timeout as number | undefined)
      })
    }
  })
}
