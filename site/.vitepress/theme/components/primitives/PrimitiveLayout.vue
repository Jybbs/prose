<script setup lang="ts">
import PrimitivesComposition from './PrimitivesComposition.vue'
import DocHeading            from '../base/DocHeading.vue'

defineProps<{ primitive: string }>()
</script>

<template>
  <PrimitivesComposition :initial-focus="primitive" />
  <slot name="lead" />
  <slot />
  <DocHeading id="related" title="Related" />
  <slot name="related" />
</template>
