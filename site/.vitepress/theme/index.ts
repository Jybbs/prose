import type { Theme }           from 'vitepress'
import DefaultTheme             from 'vitepress/theme'
import { defineAsyncComponent } from 'vue'
import type { Component }       from 'vue'

import '@shikijs/magic-move/style.css'

import '@fontsource-variable/fraunces'
import '@fontsource-variable/fraunces/wght-italic.css'
import '@fontsource-variable/jetbrains-mono'
import '@fontsource-variable/jetbrains-mono/wght-italic.css'
import '@fontsource-variable/lora'
import '@fontsource-variable/lora/wght-italic.css'

import { Tool }               from '../kit'
import { provideCurrentRule } from '../lib/composables/route'
import { Layout }             from './components/layout/Layout'

import './styles/tailwind.css'
import 'virtual:prose-palette.css'
import 'virtual:prose-tokens.css'
import './styles/tokens.css'
import './styles/accents.css'
import './styles/globals.css'
import './styles/markdown/body.css'
import './styles/markdown/callouts.css'
import './styles/markdown/headings.css'
import './styles/markdown/pull-quote.css'
import './styles/nib-stroke.css'
import './styles/panel.css'
import './styles/underline-draw.css'
import './styles/vitepress-chrome.css'

// The base and markdown sheets above stay explicit imports because their
// cascade order is load-bearing, whereas the component layer's sheets are
// order-free and load through one glob in path order. A sheet beside a base
// component loads with that component alone, which only some pages render.
import.meta.glob(['./components/**/*.css', '!./components/base/*.css', '../kit/styles/*.css'], { eager: true })

// A component carrying data that one page alone reads loads in a chunk of its
// own, registered under its file name through the deferred glob.
const components = import.meta.glob<Record<string, Component>>([
  './components/{architecture,exit-codes,fixtures,glossary,guide,integrations,reference,rules,sandbox,suppression}/*.tsx',
  '!./components/integrations/ActionsJob.tsx',
  '!./components/reference/{DiffSample,OutputSamples}.tsx'
], { eager: true })

const deferred = import.meta.glob<Record<string, Component>>([
  './components/identity/Identity{Badges,Marks,Palette,Roles,Typefaces}.tsx',
  './components/integrations/ActionsJob.tsx',
  './components/reference/{DiffSample,OutputSamples}.tsx'
])

export default {
  extends: DefaultTheme,
  Layout,
  setup() {
    provideCurrentRule()
  },
  enhanceApp({ app }) {
    for (const module of Object.values(components)) {
      for (const [name, component] of Object.entries(module)) app.component(name, component)
    }
    for (const [path, load] of Object.entries(deferred)) {
      const name = path.slice(path.lastIndexOf('/') + 1, -'.tsx'.length)
      app.component(name, defineAsyncComponent(async () => (await load())[name]))
    }
    app.component('ConfigForms', defineAsyncComponent(() => import('./components/base/ConfigForms')))
    app.component('KitCatalog', defineAsyncComponent(() => import('../kit/KitCatalog')))
    app.component('Tool', Tool)
  }
} satisfies Theme
