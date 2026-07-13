import type { Theme }     from 'vitepress'
import type { Component } from 'vue'
import DefaultTheme       from 'vitepress/theme'

import FloatingVue            from 'floating-vue'
import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import './styles/popper-vendor.css'
import '@shikijs/magic-move/style.css'
import 'virtual:group-icons.css'

import '@fontsource-variable/fraunces'
import '@fontsource-variable/fraunces/wght-italic.css'
import '@fontsource-variable/jetbrains-mono'
import '@fontsource-variable/jetbrains-mono/wght-italic.css'
import '@fontsource-variable/lora'
import '@fontsource-variable/lora/wght-italic.css'

import { stripSuffix } from '../lib/shared/strip-suffix'
import Layout          from './Layout.vue'

import 'virtual:prose-palette.css'
import './styles/tokens.css'
import './styles/accents.css'
import './styles/globals.css'
import './styles/markdown/body.css'
import './styles/markdown/callouts.css'
import './styles/markdown/headings.css'
import './styles/markdown/pull-quote.css'
import './styles/band-header.css'
import './styles/code-chip.css'
import './styles/glyph.css'
import './styles/leaders.css'
import './styles/panel.css'
import './styles/pips.css'
import './styles/popper-base.css'
import './styles/prose-mark.css'
import './styles/underline-draw.css'
import './styles/vitepress-chrome.css'

// The base and markdown sheets above stay explicit imports because their
// cascade order is load-bearing, whereas the component layer's sheets are
// order-free and load through one glob in path order.
import.meta.glob('./components/**/*.css', { eager: true })

const modules = import.meta.glob<{ default: Component }>(
  [
    './components/{exit-codes,fixtures,glossary,integrations,primitives,reference,rules,sandbox,suppression}/*.vue',
    './components/base/Tool.vue'
  ],
  { eager: true }
)
const components = Object.fromEntries(
  Object.entries(modules)
    .map(([p, mod]) => [stripSuffix(p.split('/').pop()!, '.vue'), mod.default])
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
        'az-index': {
          $extend        : 'dropdown',
          delay          : { hide: 220, show: 0 },
          distance       : 14,
          handleResize   : false,
          instantMove    : true,
          placement      : 'right-start',
          popperTriggers : ['hover'],
          triggers       : ['hover', 'focus']
        },
        glossary: {
          $extend         : 'tooltip',
          'arrow-padding' : 8,
          autoHide        : true,
          delay           : { hide: 320, show: 100 },
          handleResize    : true,
          html            : true,
          instantMove     : true,
          noAutoFocus     : true,
          placement       : 'top',
          triggers        : ['focus', 'hover']
        },
        'rule-card': {
          $extend         : 'tooltip',
          'arrow-padding' : 14,
          autoHide        : true,
          delay           : { hide: 120, show: 80 },
          handleResize    : true,
          instantMove     : true,
          noAutoFocus     : true,
          placement       : 'bottom-start',
          triggers        : ['focus', 'hover']
        },
        'run-summary-select': {
          $extend        : 'dropdown',
          delay          : { hide: 200, show: 60 },
          handleResize   : true,
          instantMove    : true,
          placement      : 'bottom-start',
          popperTriggers : ['hover'],
          triggers       : ['hover', 'focus']
        }
      }
    })
  }
} satisfies Theme
