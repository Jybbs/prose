// Reflects a fixture card's open state onto its class and the summary's
// `aria-expanded`, shared by the standalone card and the composition accordion.
export function setFixtureCardOpen(card: HTMLElement, open: boolean): void {
  card.classList.toggle('is-open', open)
  card.querySelector('.fixture-card-summary')?.setAttribute('aria-expanded', String(open))
}
