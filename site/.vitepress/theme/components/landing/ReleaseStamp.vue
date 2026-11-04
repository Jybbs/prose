<script setup lang="ts">
import type { PyPIRelease } from '../../../lib/landing/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

defineProps<{ release: PyPIRelease }>()
</script>

<template>
  <a
    :href="release.url"
    v-bind="externalAttrs(release.url)"
    class="relative flex size-132 -rotate-6 flex-col items-center justify-center gap-2 rounded-half border-2 border-dashed border-stamp-accent-65-panel-edge bg-stamp-accent-12-panel-bg text-center font-mono text-stamp-ink transition-transform hover:scale-104 hover:-rotate-3"
    :aria-label="`Current release ${release.version}, ${release.month} ${release.year}`"
  >
    <span class="pointer-events-none absolute inset-8 rounded-half border-1 border-stamp-accent-40-panel-edge" aria-hidden="true"></span>
    <span class="text-xxs tracking-display-wide text-stamp-accent-55-text-3">{{ release.month }}</span>
    <span class="font-display text-1-3rem leading-none italic">{{ release.version }}</span>
    <span class="text-0-74rem tracking-0-16em text-stamp-accent-50-text-3">'{{ release.yearShort }}</span>
  </a>
</template>
