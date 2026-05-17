import DefaultTheme from 'vitepress/theme'
import type { Theme }  from 'vitepress'
import type { App }    from 'vue'

import FloatingVue            from 'floating-vue'
import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import 'floating-vue/dist/style.css'
import 'virtual:group-icons.css'

import Layout              from './Layout.vue'
import AlignmentConfig     from './components/AlignmentConfig.vue'
import DependencyGraph     from './components/DependencyGraph.vue'
import ExitCodeMatrix      from './components/ExitCodeMatrix.vue'
import Fixture             from './components/Fixture.vue'
import GlossaryTerm        from './components/GlossaryTerm.vue'
import Kicker              from './components/Kicker.vue'
import RuleChip            from './components/RuleChip.vue'
import RuleConfigTable     from './components/RuleConfigTable.vue'
import RuleMotivation      from './components/RuleMotivation.vue'
import RulesIndex          from './components/RulesIndex.vue'
import SectionHeading      from './components/SectionHeading.vue'
import ToggleConfig        from './components/ToggleConfig.vue'
import LandingCta          from './components/landing/Cta.vue'
import LandingFeatures     from './components/landing/Features.vue'
import LandingHero         from './components/landing/Hero.vue'
import LandingMetaphor     from './components/landing/Metaphor.vue'
import LandingRulesMarquee from './components/landing/RulesMarquee.vue'
import LandingWorkflow     from './components/landing/Workflow.vue'

import './css/tokens.css'
import './css/fixture.css'
import './css/landing.css'

const COMPONENTS = {
  AlignmentConfig,
  DependencyGraph,
  ExitCodeMatrix,
  Fixture,
  GlossaryTerm,
  Kicker,
  RuleChip,
  RuleConfigTable,
  RuleMotivation,
  RulesIndex,
  SectionHeading,
  ToggleConfig,

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
          placement       : 'top',
          triggers        : ['hover', 'focus'],
          autoHide        : true,
          delay           : { show: 100, hide: 140 },
          html            : true,
          handleResize    : true,
          instantMove     : true,
          'arrow-padding' : 8
        }
      }
    })
  }
} satisfies Theme
