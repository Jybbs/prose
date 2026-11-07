import DefaultTheme        from 'vitepress/theme'
import { defineComponent } from 'vue'

import { NibSprite }          from '../../../kit'
import { useNibStroke }       from '../../../lib/composables/use-nib-stroke'
import { GlossaryFolioIndex } from '../glossary/GlossaryFolioIndex'
import { RuleChrome }         from '../rules/RuleChrome'
import { RuleLinkPopup }      from '../rules/RuleLinkPopup'
import { BuildMetadata }      from './BuildMetadata'
import { NotFound }           from './NotFound'
import { StarBadge }          from './StarBadge'

export const Layout = defineComponent({
  name: 'Layout',
  setup() {
    useNibStroke()

    return () => (
      <DefaultTheme.Layout>
        {{
          'aside-outline-after'   : () => <GlossaryFolioIndex />,
          'doc-before'            : () => <RuleChrome />,
          'layout-bottom'         : () => (
            <>
              <BuildMetadata />
              <NibSprite />
              <RuleLinkPopup />
            </>
          ),
          'nav-bar-content-after' : () => <StarBadge />,
          'not-found'             : () => <NotFound />
        }}
      </DefaultTheme.Layout>
    )
  }
})
