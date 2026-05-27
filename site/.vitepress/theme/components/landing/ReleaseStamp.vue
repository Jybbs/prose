<script setup lang="ts">
import type { PyPIRelease } from '../../../data/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

defineProps<{ release: PyPIRelease }>()
</script>

<template>
  <a
    :href="release.url"
    v-bind="externalAttrs(release.url)"
    class="landing-cta-stamp"
    :aria-label="`Current release ${release.version}, ${release.month} ${release.year}`"
  >
    <span class="landing-cta-stamp-edge" aria-hidden="true"></span>
    <span class="landing-cta-stamp-month">{{ release.month }}</span>
    <span class="landing-cta-stamp-version">{{ release.version }}</span>
    <span class="landing-cta-stamp-year">'{{ release.yearShort }}</span>
  </a>
</template>
