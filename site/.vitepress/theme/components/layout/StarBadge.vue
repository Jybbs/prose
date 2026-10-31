<script setup lang="ts">
import { externalAttrs } from '../../../lib/shared/links'
import { data as stars } from '../../../lib/shared/stars.data'
import { REPO_URL }      from '../../../lib/shared/constants'
</script>

<template>
  <a
    class="ms-6 inline-flex items-center gap-4 border-1 border-divider px-8 py-2 font-mono text-label leading-1 tracking-folio text-text-2"
    :href="REPO_URL"
    title="GitHub stars"
    v-bind="externalAttrs(REPO_URL)"
  >
    <span class="text-brand-1" aria-hidden="true">★</span>
    {{ stars.stars }}
  </a>
</template>
