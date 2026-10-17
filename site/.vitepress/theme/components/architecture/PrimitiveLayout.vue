<script setup lang="ts">
import PrimitivesComposition from './PrimitivesComposition.vue'
import DocHeading            from '../base/DocHeading.vue'

import type { PrimitiveSlug } from '../../../lib/shared/registries'

defineProps<{ primitive: PrimitiveSlug }>()
</script>

<template>
  <PrimitivesComposition :initial-focus="primitive" />
  <slot name="lead" />
  <slot />
  <DocHeading id="related" title="Related" />
  <slot name="related" />
</template>
