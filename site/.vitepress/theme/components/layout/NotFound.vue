<script setup lang="ts">
import Kicker  from '../base/Kicker.vue'
import LinkRow from '../base/LinkRow.vue'

const links = [
  { href: '/',                    text: 'Home'         },
  { href: '/guide/installation',  text: 'Installation' },
  { href: '/rules/',              text: 'Rules'        },
  { href: '/primitives/source',   text: 'Primitives'   }
]
</script>

<template>
  <div class="not-found">
    <Kicker class="not-found-kicker">404</Kicker>
    <h1 class="not-found-headline"><strong>A</strong> page <em>misaligned</em>.</h1>
    <p class="not-found-body">
      The page you wanted didn't make it through the proof. Try one of the
      surfaces below, or jump back to the catalog and pick another rule.
    </p>
    <LinkRow :items="links" variant="not-found-links" />
  </div>
</template>
