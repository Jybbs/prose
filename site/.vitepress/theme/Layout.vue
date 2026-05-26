<script setup lang="ts">
import DefaultTheme    from 'vitepress/theme'
import { watchEffect } from 'vue'

import { provideCurrentRule, useCurrentFamily } from '../lib/composables/route'

import BuildMetadata      from './components/layout/BuildMetadata.vue'
import GlossaryFolioIndex from './components/glossary/GlossaryFolioIndex.vue'
import NotFound           from './components/layout/NotFound.vue'
import RuleChrome         from './components/rules/RuleChrome.vue'
import StarBadge          from './components/layout/StarBadge.vue'

provideCurrentRule()
const family = useCurrentFamily()
watchEffect(() => {
  if (typeof document === 'undefined') return
  if (family.value) document.body.setAttribute('data-family', family.value)
  else              document.body.removeAttribute('data-family')
})
</script>

<template>
  <DefaultTheme.Layout>
    <template #nav-bar-content-after>
      <StarBadge />
    </template>
    <template #doc-before>
      <RuleChrome />
    </template>
    <template #aside-outline-after>
      <GlossaryFolioIndex />
    </template>
    <template #layout-bottom>
      <BuildMetadata />
    </template>
    <template #not-found>
      <NotFound />
    </template>
  </DefaultTheme.Layout>
</template>
