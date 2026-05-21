import type { Component, Theme } from 'vitepress'
import DefaultTheme              from 'vitepress/theme'

import FloatingVue            from 'floating-vue'
import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import 'floating-vue/dist/style.css'
import 'shiki-magic-move/style.css'
import 'virtual:group-icons.css'

import '@fontsource-variable/fraunces'
import '@fontsource-variable/fraunces/wght-italic.css'
import '@fontsource-variable/jetbrains-mono'
import '@fontsource-variable/jetbrains-mono/wght-italic.css'
import '@fontsource-variable/lora'
import '@fontsource-variable/lora/wght-italic.css'

import Layout from './Layout.vue'

import './styles/tokens.css'
import './styles/accents.css'
import './styles/globals.css'
import './styles/markdown/body.css'
import './styles/markdown/callouts.css'
import './styles/markdown/headings.css'
import './styles/primitives.css'
import './styles/prose-mark.css'
import './styles/vitepress-chrome.css'

import './components/aside/fixture-toc.css'
import './components/aside/related-rules.css'
import './components/base/category-chip.css'
import './components/base/chips.css'
import './components/base/disclosure.css'
import './components/base/family-chip.css'
import './components/base/kicker.css'
import './components/exit-codes/exit-code-matrix.css'
import './components/fixtures/fixture-landing.css'
import './components/fixtures/fixture.css'
import './components/glossary/glossary-index.css'
import './components/glossary/glossary.css'
import './components/integrations/editor-run-on-save.css'
import './components/landing/built-on.css'
import './components/landing/cta.css'
import './components/landing/hero.css'
import './components/landing/landing.css'
import './components/landing/metaphor.css'
import './components/landing/surfaces/surface-card-base.css'
import './components/landing/surfaces/surfaces.css'
import './components/landing/typing-demo.css'
import './components/landing/workflow.css'
import './components/layout/build-metadata.css'
import './components/layout/not-found.css'
import './components/layout/star-badge.css'
import './components/primitives/dependency-graph.css'
import './components/primitives/primitives-composition.css'
import './components/rules/pipeline-order.css'
import './components/rules/related-rules-inline.css'
import './components/rules/rule-card-grid.css'
import './components/rules/rules-index.css'
import './components/rules/rules-plate.css'

const modules = import.meta.glob<{ default: Component }>(
  ['./components/{exit-codes,fixtures,glossary,integrations,primitives,rules}/*.vue', './components/base/Tool.vue'],
  { eager: true }
)
const components = Object.fromEntries(
  Object.entries(modules)
    .map(([p, mod]) => [p.split('/').pop()!.replace(/\.vue$/, ''), mod.default])
)

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    enhanceAppWithTabs(app)
    for (const [name, component] of Object.entries(components).sort()) {
      app.component(name, component)
    }
    app.use(FloatingVue, {
      themes: {
        glossary: {
          $extend         : 'tooltip',
          'arrow-padding' : 8,
          autoHide        : true,
          delay           : { hide: 140, show: 100 },
          handleResize    : true,
          html            : true,
          instantMove     : true,
          placement       : 'top',
          triggers        : ['focus', 'hover']
        }
      }
    })
  }
} satisfies Theme
