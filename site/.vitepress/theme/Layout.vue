<script setup lang="ts">
import DefaultTheme        from 'vitepress/theme'
import { watchEffect }     from 'vue'

import { useCurrentDomain } from '../lib/shared/route'

import BuildMetadata from './components/layout/BuildMetadata.vue'
import FixtureToc    from './components/aside/FixtureToc.vue'
import NotFound      from './components/layout/NotFound.vue'
import RelatedRules  from './components/aside/RelatedRules.vue'
import RuleChrome    from './components/aside/RuleChrome.vue'
import StarBadge     from './components/layout/StarBadge.vue'

const domain = useCurrentDomain()

watchEffect(() => {
  if (typeof document === 'undefined') return
  if (domain.value) document.body.setAttribute('data-domain', domain.value)
  else              document.body.removeAttribute('data-domain')
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
    <template #aside-top>
      <FixtureToc />
    </template>
    <template #aside-outline-after>
      <RelatedRules />
    </template>
    <template #layout-bottom>
      <BuildMetadata />
    </template>
    <template #not-found>
      <NotFound />
    </template>
  </DefaultTheme.Layout>
</template>
