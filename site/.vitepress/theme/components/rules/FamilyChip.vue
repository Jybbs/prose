<script setup lang="ts">
import { computed } from 'vue'

import Chip     from '../base/Chip.vue'
import NibBadge from '../base/NibBadge.vue'

import { useCurrentRule } from '../../../lib/composables/route'
import { familyEmblem }   from '../../../lib/shared/emblems'
import * as registries    from '../../../lib/shared/registries'

const props = withDefaults(defineProps<{
  family ?: registries.GlossaryFamily
  linked ?: boolean
}>(), { linked: true })

const rule   = useCurrentRule()
const family = computed(() => props.family ?? rule.value?.family ?? null)
const emblem = computed(() => family.value && familyEmblem(family.value))
const href   = computed(() => props.linked && family.value && family.value in registries.FAMILY_META
  ? `/rules/${family.value}/`
  : undefined)
</script>

<template>
  <Chip v-if="emblem" class="nib-ground" variant="family-chip" :family="emblem.family" :href="href">
    <NibBadge :emblem="emblem" :size="15.2" />
    <span>{{ emblem.name }}</span>
  </Chip>
</template>
