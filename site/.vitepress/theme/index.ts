import type { Theme } from 'vitepress'
import DefaultTheme   from 'vitepress/theme'
import type { App }   from 'vue'

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

import Layout             from './Layout.vue'
import BuiltOn            from './components/landing/BuiltOn.vue'
import CompositionGrid    from './components/rules/CompositionGrid.vue'
import DependencyGraph    from './components/primitives/DependencyGraph.vue'
import ExitCodeMatrix     from './components/exit-codes/ExitCodeMatrix.vue'
import Fixture            from './components/fixtures/Fixture.vue'
import GlossaryIndex      from './components/glossary/GlossaryIndex.vue'
import GlossaryTerm       from './components/glossary/GlossaryTerm.vue'
import PipelineOrder      from './components/rules/PipelineOrder.vue'
import RelatedRulesInline from './components/rules/RelatedRulesInline.vue'
import RuleCardGrid       from './components/rules/RuleCardGrid.vue'
import RuleChip           from './components/rules/RuleChip.vue'
import RuleConfigTable    from './components/rules/RuleConfigTable.vue'
import RulesIndex         from './components/rules/RulesIndex.vue'
import Tool               from './components/base/Tool.vue'

import './styles/tokens.css'
import './styles/accents.css'
import './styles/globals.css'
import './styles/markdown-body.css'
import './styles/markdown-callouts.css'
import './styles/markdown-headings.css'
import './styles/primitives.css'
import './styles/vitepress-chrome.css'

import './components/aside/category-chip.css'
import './components/aside/domain-chip.css'
import './components/aside/fixture-toc.css'
import './components/aside/related-rules.css'
import './components/base/chips.css'
import './components/base/disclosure.css'
import './components/exit-codes/exit-code-matrix.css'
import './components/fixtures/fixture.css'
import './components/fixtures/fixture-landing.css'
import './components/glossary/glossary-index.css'
import './components/glossary/glossary.css'
import './components/landing/built-on.css'
import './components/landing/carousel.css'
import './components/landing/landing.css'
import './components/landing/typing-demo.css'
import './components/layout/build-metadata.css'
import './components/layout/not-found.css'
import './components/primitives/dependency-graph.css'
import './components/rules/related-rules-inline.css'
import './components/rules/rule-card-grid.css'
import './components/rules/rules-index.css'

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    enhanceAppWithTabs(app)
    app.component('BuiltOn',            BuiltOn)
    app.component('CompositionGrid',    CompositionGrid)
    app.component('DependencyGraph',    DependencyGraph)
    app.component('ExitCodeMatrix',     ExitCodeMatrix)
    app.component('Fixture',            Fixture)
    app.component('GlossaryIndex',      GlossaryIndex)
    app.component('GlossaryTerm',       GlossaryTerm)
    app.component('PipelineOrder',      PipelineOrder)
    app.component('RelatedRulesInline', RelatedRulesInline)
    app.component('RuleCardGrid',       RuleCardGrid)
    app.component('RuleChip',           RuleChip)
    app.component('RuleConfigTable',    RuleConfigTable)
    app.component('RulesIndex',         RulesIndex)
    app.component('Tool',               Tool)
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
