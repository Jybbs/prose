import DefaultTheme from 'vitepress/theme'
import type { Theme }  from 'vitepress'
import type { App }    from 'vue'

import FloatingVue            from 'floating-vue'
import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import 'floating-vue/dist/style.css'
import 'virtual:group-icons.css'

import Layout              from './Layout.vue'
import DependencyGraph     from './components/DependencyGraph.vue'
import ExitCodeMatrix      from './components/ExitCodeMatrix.vue'
import Fixture             from './components/Fixture.vue'
import GlossaryTerm        from './components/GlossaryTerm.vue'
import RuleChip            from './components/RuleChip.vue'
import RuleConfigTable     from './components/RuleConfigTable.vue'
import RuleMotivation      from './components/RuleMotivation.vue'
import RulesIndex          from './components/RulesIndex.vue'
import SectionHeading      from './components/SectionHeading.vue'
import LandingCta          from './components/landing/Cta.vue'
import LandingFeatures     from './components/landing/Features.vue'
import LandingHero         from './components/landing/Hero.vue'
import LandingMetaphor     from './components/landing/Metaphor.vue'
import LandingRulesMarquee from './components/landing/RulesMarquee.vue'
import LandingWorkflow     from './components/landing/Workflow.vue'

import './css/tokens.css'
import './css/globals.css'
import './css/fixture.css'
import './css/landing.css'

const COMPONENTS = {
  DependencyGraph,
  ExitCodeMatrix,
  Fixture,
  GlossaryTerm,
  RuleChip,
  RuleConfigTable,
  RuleMotivation,
  RulesIndex,
  SectionHeading,

  LandingCta,
  LandingFeatures,
  LandingHero,
  LandingMetaphor,
  LandingRulesMarquee,
  LandingWorkflow
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
