import { SCHEMES }                              from '../../lib/shared/palette'
import { huesIn, pointAt, renderTest, styleOf } from './page'

const SURFACES = [
  { prop: 'color',             route: '/reference/exit-codes',             selector: '.exit-codes-band-head .band-name',                surface: 'an exit-code band name' },
  { prop: 'color',             route: '/reference/exit-codes',             selector: '.exit-code-index-row:not([aria-current]) > span', surface: 'an unselected exit-code numeral' },
  { prop: 'color',             route: '/reference/exit-codes',             selector: '.exit-code-entry-facts dt',                       surface: 'an exit-code fact label' },
  { prop: 'color',             route: '/architecture/',                    selector: '.band-name',                                      surface: 'an architecture band name' },
  { prop: 'background-color',  route: '/architecture/',                    selector: '.hairline',                                       surface: 'an architecture band rule' },
  { prop: 'background-color',  route: '/architecture/',                    selector: 'section[data-layer] li',                          surface: 'an architecture tile' },
  { prop: 'color',             route: '/reference/',                       selector: '.az-index-section-head .kicker',                  surface: 'a reference section label' },
  { prop: 'color',             route: '/reference/',                       selector: '.az-index-section-head .kicker + span',           surface: 'a reference section count' },
  { prop: 'color',             route: '/reference/suppression-directives', selector: '[data-role="comment"] .directive-anatomy-part',   surface: 'the directive anatomy comment' },
  { prop: 'color',             route: '/reference/suppression-directives', selector: '[data-role="namespace"] .directive-anatomy-part', surface: 'a directive anatomy part' },
  { prop: 'color',             route: '/reference/suppression-directives', selector: '[data-role="namespace"] > span:last-child',       surface: 'a directive anatomy gloss' },
  { prop: 'background-color',  route: '/reference/suppression-directives', selector: '[data-role="namespace"] > span:nth-child(2)',     surface: 'a directive anatomy tick' },
  { prop: 'color',             route: '/reference/suppression-directives', selector: '.anatomy-band-name',                              surface: 'a directive anatomy band name' },
  { prop: 'background-color',  route: '/reference/suppression-directives', selector: '.anatomy-band-rule',                              surface: 'a directive anatomy band rule' },
  { prop: 'border-top-color',  route: '/reference/suppression-directives', selector: '.directive-anatomy-plate',                        surface: 'the rule above the directive anatomy plate' },
  { prop: 'border-top-color',  route: '/reference/suppression-directives', selector: '.directive-anatomy-thumb',                        surface: 'the edge of a directive anatomy index entry' },
  { prop: 'background-color',  route: '/sandbox/',                         selector: '.sandbox-ruler-marker',                           surface: 'a sandbox ruler needle', pseudo: '::before' },
  { prop: 'color',             route: '/sandbox/',                         selector: '.sandbox-ruler-chip > span:last-child',           surface: 'the figure in a default sandbox ruler chip' },
  { prop: 'color',             route: '/sandbox/',                         selector: '.sandbox-tags-tag button > span:last-child',      surface: 'a sandbox rule tag slug' },
  { prop: 'color',             route: '/guide/quick-start',                selector: '.vp-doc h2 .header-anchor',                       surface: 'a hovered heading permalink', point: '.vp-doc h2' },
  { prop: 'color',             route: '/guide/quick-start',                selector: '.vp-doc h1 .header-anchor',                       surface: 'a hovered title permalink', point: '.vp-doc h1' },
  { prop: 'color',             route: '/reference/configuration',          selector: '.per-rule-facets-rule-head .header-anchor',       surface: 'a per-rule head permalink' },
  { prop: 'color',             route: '/reference/configuration',          selector: '.per-rule-facets-term .header-anchor',            surface: 'a per-rule term permalink' },
  { prop: 'color',             route: '/guide/quick-start',                selector: '.VPSidebarItem.is-active > .item .link > .text',  surface: 'the current sidebar item' },
  { prop: 'border-top-color',  route: '/reference/glossary',               selector: 'div:has(> input[type="search"])',                 surface: 'a hovered glossary search field', point: 'input' },
  { prop: 'border-left-color', route: '/guide/',                           selector: '.pq-row .pq-aside',                               surface: 'the edge of a pull quote aside' },
  { prop: 'scrollbar-color',   route: '/guide/quick-start',                selector: 'div[class*="language-"] pre',                     surface: 'the scroll thumb of a code block' },
  { prop: 'border-top-color',  route: '/guide/quick-start',                selector: 'div[class*="language-"] > button.copy',           surface: 'the edge of a copy button' }
]

describe.each(SCHEMES)('in the %s scheme', scheme => {
  renderTest.for(SURFACES)('$surface stays neutral', async ({ point, prop, pseudo, route, selector }, { visit }) => {
    const page = await visit(route, { scheme })

    if (point) await pointAt(page.locator(point).first())

    expect(huesIn(await styleOf(page.locator(selector).first(), prop, pseudo))).toStrictEqual([])
  })
})
