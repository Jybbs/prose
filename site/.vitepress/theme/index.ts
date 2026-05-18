import DefaultTheme from 'vitepress/theme'
import type { Theme }  from 'vitepress'
import type { App }    from 'vue'

import FloatingVue            from 'floating-vue'
import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import 'floating-vue/dist/style.css'
import 'virtual:group-icons.css'

import '@fontsource-variable/fraunces'
import '@fontsource-variable/fraunces/wght-italic.css'
import '@fontsource-variable/lora'
import '@fontsource-variable/lora/wght-italic.css'
import '@fontsource-variable/jetbrains-mono'
import '@fontsource-variable/jetbrains-mono/wght-italic.css'

import Layout          from './Layout.vue'
import DependencyGraph from './components/DependencyGraph.vue'
import ExitCodeMatrix  from './components/ExitCodeMatrix.vue'
import Fixture         from './components/Fixture.vue'
import GlossaryTerm    from './components/GlossaryTerm.vue'
import RuleChip        from './components/RuleChip.vue'
import RuleConfigTable from './components/RuleConfigTable.vue'
import RuleMotivation  from './components/RuleMotivation.vue'
import RulesIndex      from './components/RulesIndex.vue'

import './css/tokens.css'
import './css/globals.css'

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
