<script setup lang="ts">
import DefaultTheme from 'vitepress/theme'

import { provideCurrentRule, provideCurrentSlugs } from '../lib/composables/route'
import { useFamilyBodyAttr }                       from '../lib/composables/use-family-body-attr'

import BuildMetadata from './components/layout/BuildMetadata.vue'
import NotFound      from './components/layout/NotFound.vue'
import RelatedRules  from './components/aside/RelatedRules.vue'
import RuleChrome    from './components/aside/RuleChrome.vue'
import StarBadge     from './components/layout/StarBadge.vue'

provideCurrentSlugs()
provideCurrentRule()
useFamilyBodyAttr()
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
