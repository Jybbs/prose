<script setup lang="ts">
import DefaultTheme from 'vitepress/theme'

import { useNibStroke } from '../lib/composables/use-nib-stroke'

import { NibSprite }          from '../kit'
import { BuildMetadata }      from './components/layout/BuildMetadata'
import { GlossaryFolioIndex } from './components/glossary/GlossaryFolioIndex'
import { NotFound }           from './components/layout/NotFound'
import { RuleChrome }         from './components/rules/RuleChrome'
import { RuleLinkPopup }      from './components/rules/RuleLinkPopup'
import { StarBadge }          from './components/layout/StarBadge'

useNibStroke()
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
      <NibSprite />
      <RuleLinkPopup />
    </template>
    <template #not-found>
      <NotFound />
    </template>
  </DefaultTheme.Layout>
</template>
