<script setup lang="ts">
interface Link {
  href : string
  text : string
}

const links: Link[] = [
  { href: '/guide/configuration', text: 'Configuration' },
  { href: '/guide/installation',  text: 'Installation'  },
  { href: '/primitives/source',   text: 'Primitives'    },
  { href: '/rules/',              text: 'Rules catalog' }
]
</script>

<template>
  <div class="landing-cta">
    <p class="landing-kicker">The Page</p>
    <h2>Ready to <em>read</em>?</h2>
    <div class="landing-links">
      <a v-for="link in links" :key="link.href" :href="link.href">{{ link.text }}</a>
    </div>
  </div>
</template>
