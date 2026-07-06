// Renders both VDropdown slots inline so popper content is reachable without
// floating-vue's real positioning.
export const popperStubMount = {
  directives : { 'close-popper': {} },
  stubs      : { VDropdown: { template: '<div><slot /><slot name="popper" /></div>' } }
}
