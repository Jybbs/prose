interface ChipRule {
  family : string
  slug   : string
}

// A constant cross-family consumer list, dense enough that adjacent chips
// clash hues the way the crate-internal primitives view does. Every variation
// renders this same corpus, so the comparison is purely about chip treatment.
export const CHIPS: ChipRule[] = [
  { family : 'alignment',  slug : 'align-equals'          },
  { family : 'ordering',   slug : 'alphabetize'           },
  { family : 'formatting', slug : 'collection-layout'     },
  { family : 'docs',       slug : 'docstring-wrap'        },
  { family : 'lint',       slug : 'single-use-variables'  },
  { family : 'alignment',  slug : 'align-colons'          },
  { family : 'formatting', slug : 'strip-trailing-commas' },
  { family : 'lint',       slug : 'reassigned-constants'  },
  { family : 'alignment',  slug : 'align-imports'         },
  { family : 'docs',       slug : 'docstring-frame'       },
  { family : 'formatting', slug : 'blank-lines'           },
  { family : 'lint',       slug : 'legacy-union-syntax'   },
  { family : 'alignment',  slug : 'align-match-case'      },
  { family : 'formatting', slug : 'signature-layout'      },
]
