import { defineComponent } from 'vue'
import type { VNodeChild } from 'vue'

import { MOTIFS, strokesOf } from '../../lib/shared/emblems'
import type { Fit }         from '../../lib/shared/emblems'

import * as nib from './nib'

const paths = (drawn: readonly string[]) => drawn.map(d => <path d={d} key={d} />)

const offsets = (fit: Fit, drawn: string, placed: readonly number[]) => placed.map(offset => (
  <use href={`#${drawn}`} key={offset} transform={nib.diagonal(fit, offset)} />
))

const fitted = (fit: Fit, units: number, drawing: VNodeChild) => (
  <g transform={nib.fitTransform(fit)}>
    <g {...nib.STROKE} stroke-width={nib.perScale(fit, units)}>{drawing}</g>
  </g>
)

// Declares every motif's paths, jitter filter, and sized symbols once per
// page, for each `NibBadge` to reference.
export const NibSprite = defineComponent({
  name : 'NibSprite',
  setup() {
    return () => (
      <svg aria-hidden="true" class="absolute size-0 overflow-hidden" focusable="false">
        <defs>
          {MOTIFS.flatMap((motif, at) => {
            const drawn  = nib.nibId(motif, 'paths')
            const jitter = nib.nibId(motif, 'jitter')
            return [
              <g id={drawn} key={drawn}>{paths(strokesOf(motif))}</g>,
              <filter height="140%" id={jitter} key={jitter} width="140%" x="-20%" y="-20%">
                <feTurbulence
                  baseFrequency={nib.NIB.jitter.baseFrequency}
                  numOctaves={nib.NIB.jitter.numOctaves}
                  result="noise"
                  seed={nib.NIB.jitter.seed + at * nib.NIB.jitter.seedStep}
                  type="fractalNoise"
                />
                <feDisplacementMap
                  in="SourceGraphic"
                  in2="noise"
                  scale={nib.NIB.jitter.scale}
                  xChannelSelector="R"
                  yChannelSelector="G"
                />
              </filter>,
              <symbol id={nib.nibId(motif, 'small')} key={`${drawn}-small`} overflow="visible" viewBox="0 0 24 24">
                {motif.small
                  ? fitted(motif.small.fit, nib.NIB.stroke.small, [
                    paths(motif.small.lines),
                    motif.small.heavy?.map(d => (
                      <path d={d} key={d} stroke-width={nib.perScale(motif.small!.fit, nib.NIB.stroke.heavy)} />
                    )),
                    motif.small.dots?.map(([cx, cy, r]) => (
                      <circle cx={cx} cy={cy} fill="currentColor" key={`${cx} ${cy}`} r={r} stroke="none" />
                    ))
                  ])
                  : fitted(motif.fit, nib.NIB.stroke.nib, offsets(motif.fit, drawn, nib.NIB.small))}
              </symbol>,
              <symbol id={nib.nibId(motif, 'large')} key={`${drawn}-large`} overflow="visible" viewBox="0 0 24 24">
                <g filter={`url(#${jitter})`}>
                  {fitted(motif.fit, nib.NIB.stroke.nib, offsets(motif.fit, drawn, nib.NIB.large))}
                </g>
              </symbol>
            ]
          })}
        </defs>
      </svg>
    )
  }
})
