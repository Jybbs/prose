<script setup lang="ts">
import DefaultTheme from 'vitepress/theme'

import BuildMetadata      from './components/layout/BuildMetadata.vue'
import GlossaryFolioIndex from './components/glossary/GlossaryFolioIndex.vue'
import NotFound           from './components/layout/NotFound.vue'
import RuleChrome         from './components/rules/RuleChrome.vue'
import RuleLinkPopper     from './components/rules/RuleLinkPopper.vue'
import StarBadge          from './components/layout/StarBadge.vue'
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
      <RuleLinkPopper />
    </template>
    <template #not-found>
      <NotFound />
    </template>
  </DefaultTheme.Layout>
</template>
