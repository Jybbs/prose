<script setup lang="ts">
import DefaultTheme from 'vitepress/theme'

import BuildMetadata from './components/BuildMetadata.vue'
import CategoryChip  from './components/aside/CategoryChip.vue'
import FixtureToc    from './components/aside/FixtureToc.vue'
import NotFound      from './components/NotFound.vue'
import RelatedRules  from './components/aside/RelatedRules.vue'
</script>

<template>
  <DefaultTheme.Layout>
    <template #doc-before>
      <CategoryChip />
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
