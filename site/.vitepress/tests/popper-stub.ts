// Stands in for floating-vue without its positioning. `VDropdown` renders both
// slots inline so popper content is reachable, whereas `VTooltip` renders the
// trigger alone, and the directives are inert.
export const popperStubMount = {
  directives : { 'close-popper': {}, tooltip: {} },
  stubs      : {
    VDropdown : { template: '<div><slot /><slot name="popper" /></div>' },
    VTooltip  : { template: '<div><slot /></div>' }
  }
}
