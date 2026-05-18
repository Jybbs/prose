<script setup lang="ts">
import { computed } from 'vue'

import Chip from '../base/Chip.vue'

import { CATEGORY_META }  from '../../../lib/shared/registries'
import { useCurrentRule } from '../../../lib/shared/route'

const rule = useCurrentRule()
const meta = computed(() => rule.value && CATEGORY_META[rule.value.category])
</script>

<template>
  <Chip v-if="rule && meta" variant="category-chip" :category="rule.category">
    <span class="category-chip-badge" aria-hidden="true">{{ meta.badge }}</span>
    <span class="category-chip-label">{{ meta.label }}</span>
  </Chip>
</template>
