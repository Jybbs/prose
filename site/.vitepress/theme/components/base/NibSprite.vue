<script setup lang="ts">
import { MOTIFS } from '../../../lib/shared/emblems'
import * as nib   from '../../../lib/shared/nib'

const motifs = MOTIFS.map((motif, i) => ({
  drawn  : [...motif.shapes, ...motif.cuts, ...motif.lines],
  jitter : nib.nibId(motif, 'jitter'),
  large  : nib.nibId(motif, 'large'),
  motif  : motif,
  paths  : nib.nibId(motif, 'paths'),
  seed   : nib.NIB.jitter.seed + i * nib.NIB.jitter.seedStep,
  small  : nib.nibId(motif, 'small')
}))
</script>

<template>
  <svg class="nib-sprite" aria-hidden="true" focusable="false">
    <defs>
      <template v-for="m in motifs" :key="m.paths">
        <g :id="m.paths">
          <path v-for="d in m.drawn" :key="d" :d="d" />
        </g>
        <filter :id="m.jitter" x="-20%" y="-20%" width="140%" height="140%">
          <feTurbulence
            type="fractalNoise"
            :baseFrequency="nib.NIB.jitter.baseFrequency"
            :numOctaves="nib.NIB.jitter.numOctaves"
            :seed="m.seed"
            result="noise"
          />
          <feDisplacementMap
            in="SourceGraphic"
            in2="noise"
            :scale="nib.NIB.jitter.scale"
            xChannelSelector="R"
            yChannelSelector="G"
          />
        </filter>
        <symbol :id="m.small" viewBox="0 0 24 24" overflow="visible">
          <g v-if="m.motif.small" :transform="nib.fitTransform(m.motif.small.fit)">
            <g v-bind="nib.STROKE" :stroke-width="nib.perScale(m.motif.small.fit, nib.NIB.stroke.small)">
              <path v-for="d in m.motif.small.lines" :key="d" :d="d" />
              <path
                v-for="d in m.motif.small.heavy"
                :key="d"
                :d="d"
                :stroke-width="nib.perScale(m.motif.small.fit, nib.NIB.stroke.heavy)"
              />
              <circle
                v-for="[cx, cy, r] in m.motif.small.dots"
                :key="`${cx} ${cy}`"
                :cx="cx"
                :cy="cy"
                :r="r"
                fill="currentColor"
                stroke="none"
              />
            </g>
          </g>
          <g v-else :transform="nib.fitTransform(m.motif.fit)">
            <g v-bind="nib.STROKE" :stroke-width="nib.perScale(m.motif.fit, nib.NIB.stroke.nib)">
              <use
                v-for="offset in nib.NIB.small"
                :key="offset"
                :href="`#${m.paths}`"
                :transform="nib.diagonal(m.motif.fit, offset)"
              />
            </g>
          </g>
        </symbol>
        <symbol :id="m.large" viewBox="0 0 24 24" overflow="visible">
          <g :filter="`url(#${m.jitter})`">
            <g :transform="nib.fitTransform(m.motif.fit)">
              <g v-bind="nib.STROKE" :stroke-width="nib.perScale(m.motif.fit, nib.NIB.stroke.nib)">
                <use
                  v-for="offset in nib.NIB.large"
                  :key="offset"
                  :href="`#${m.paths}`"
                  :transform="nib.diagonal(m.motif.fit, offset)"
                />
              </g>
            </g>
          </g>
        </symbol>
      </template>
    </defs>
  </svg>
</template>
