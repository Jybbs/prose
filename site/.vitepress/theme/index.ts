import DefaultTheme from 'vitepress/theme'
import type { Theme }  from 'vitepress'
import type { App }    from 'vue'
import { onMounted }   from 'vue'

import { enhanceAppWithTabs } from 'vitepress-plugin-tabs/client'

import 'virtual:group-icons.css'

import Layout              from './Layout.vue'
import AlignmentConfig     from './components/AlignmentConfig.vue'
import CopyBlock           from './components/CopyBlock.vue'
import DependencyGraph     from './components/DependencyGraph.vue'
import ExitCodeMatrix      from './components/ExitCodeMatrix.vue'
import Fixture             from './components/Fixture.vue'
import Glossary            from './components/Glossary.vue'
import Kbd                 from './components/Kbd.vue'
import RuleChip            from './components/RuleChip.vue'
import RuleMotivation      from './components/RuleMotivation.vue'
import RulesIndex          from './components/RulesIndex.vue'
import ToggleConfig        from './components/ToggleConfig.vue'
import VersionedSnippet    from './components/VersionedSnippet.vue'
import LandingBeforeAfter  from './components/landing/BeforeAfter.vue'
import LandingCta          from './components/landing/Cta.vue'
import LandingFeatures     from './components/landing/Features.vue'
import LandingHero         from './components/landing/Hero.vue'
import LandingMetaphor     from './components/landing/Metaphor.vue'
import LandingRulesMarquee from './components/landing/RulesMarquee.vue'
import LandingWorkflow     from './components/landing/Workflow.vue'

import { data as starsData } from '../data/stars.data'

import './css/tokens.css'
import './css/fixture.css'
import './css/landing.css'

const COMPONENTS = {
  AlignmentConfig,
  CopyBlock,
  DependencyGraph,
  ExitCodeMatrix,
  Fixture,
  Glossary,
  Kbd,
  LandingBeforeAfter,
  LandingCta,
  LandingFeatures,
  LandingHero,
  LandingMetaphor,
  LandingRulesMarquee,
  LandingWorkflow,
  RuleChip,
  RuleMotivation,
  RulesIndex,
  ToggleConfig,
  VersionedSnippet
}

function registerComponents(app: App) {
  for (const [name, component] of Object.entries(COMPONENTS)) {
    app.component(name, component)
  }
}

function attachStarsToNav() {
  const githubLink = document.querySelector(
    '.VPSocialLinks a[href*="github.com/Jybbs/prose"]'
  )
  if (!githubLink || githubLink.querySelector('.star-count')) return
  const badge = document.createElement('span')
  badge.className = 'star-count'
  badge.title = 'GitHub stars'
  const glyph = document.createElement('span')
  glyph.className = 'star-glyph'
  glyph.setAttribute('aria-hidden', 'true')
  glyph.textContent = '★'
  badge.append(glyph, ` ${starsData.stars}`)
  githubLink.appendChild(badge)
}

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    enhanceAppWithTabs(app)
    registerComponents(app)
  },
  setup() {
    onMounted(() => {
      attachStarsToNav()
      setTimeout(attachStarsToNav, 120)
      new MutationObserver(attachStarsToNav).observe(document.body, { childList: true, subtree: true })
    })
  }
} satisfies Theme
