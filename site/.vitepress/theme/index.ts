import type { Theme }           from 'vitepress'
import DefaultTheme             from 'vitepress/theme'
import { defineAsyncComponent } from 'vue'
import type { Component }       from 'vue'

import '@shikijs/magic-move/style.css'
import 'virtual:group-icons.css'

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
// order-free and load through one glob in path order.
import.meta.glob(['./components/**/*.css', '../kit/styles/*.css'], { eager: true })

const components = import.meta.glob<Record<string, Component>>(
  './components/{architecture,exit-codes,fixtures,glossary,guide,identity,integrations,reference,rules,sandbox,suppression}/*.tsx',
  { eager: true }
)

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
    app.component('KitCatalog', defineAsyncComponent(() => import('../kit/KitCatalog')))
    app.component('Tool', Tool)
  }
} satisfies Theme
