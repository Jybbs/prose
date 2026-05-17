<script setup lang="ts">
import { data as rules } from '../../data/rules.data'
import { data as build } from '../../data/build.data'
</script>

<template>
  <footer class="build-metadata">
    <span class="build-metadata-item">
      <span class="build-metadata-label">Version</span>
      <span class="build-metadata-value">{{ build.version }}</span>
    </span>
    <span class="build-metadata-item">
      <span class="build-metadata-label">Commit</span>
      <span class="build-metadata-value"><code>{{ build.gitSha }}</code></span>
    </span>
    <span class="build-metadata-item">
      <span class="build-metadata-label">Rules</span>
      <span class="build-metadata-value">{{ rules.length }}</span>
    </span>
    <span class="build-metadata-item">
      <span class="build-metadata-label">Fixtures</span>
      <span class="build-metadata-value">{{ build.fixtureCount }}</span>
    </span>
    <span class="build-metadata-item">
      <span class="build-metadata-label">Built</span>
      <span class="build-metadata-value">{{ build.builtAt }}</span>
    </span>
  </footer>
</template>
