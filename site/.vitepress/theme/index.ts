import type { Theme } from 'vitepress'
import DefaultTheme   from 'vitepress/theme'
import type { App }   from 'vue'

import FloatingVue            from 'floating-vue'
import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import 'floating-vue/dist/style.css'
import 'virtual:group-icons.css'

import '@fontsource-variable/fraunces'
import '@fontsource-variable/fraunces/wght-italic.css'
import '@fontsource-variable/jetbrains-mono'
import '@fontsource-variable/jetbrains-mono/wght-italic.css'
import '@fontsource-variable/lora'
import '@fontsource-variable/lora/wght-italic.css'

import Layout          from './Layout.vue'
import ExitCodeMatrix  from './components/exit-codes/ExitCodeMatrix.vue'
import Fixture         from './components/fixtures/Fixture.vue'
import GlossaryTerm    from './components/glossary/GlossaryTerm.vue'
import DependencyGraph from './components/primitives/DependencyGraph.vue'
import RuleChip        from './components/rules/RuleChip.vue'
import RuleConfigTable from './components/rules/RuleConfigTable.vue'
import RuleMotivation  from './components/rules/RuleMotivation.vue'
import RulesIndex      from './components/rules/RulesIndex.vue'

import './styles/tokens.css'
import './styles/globals.css'
import './styles/markdown-body.css'
import './styles/markdown-callouts.css'
import './styles/markdown-headings.css'
import './styles/primitives.css'
import './styles/vitepress-chrome.css'

const COMPONENTS = {
  DependencyGraph,
  ExitCodeMatrix,
  Fixture,
  GlossaryTerm,
  RuleChip,
  RuleConfigTable,
  RuleMotivation,
  RulesIndex
}

function registerComponents(app: App) {
  for (const [name, component] of Object.entries(COMPONENTS)) {
    app.component(name, component)
  }
}

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    enhanceAppWithTabs(app)
    registerComponents(app)
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
