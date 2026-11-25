describe('virtual:group-icons.css', () => {
  it('resolves in the docs project, so a component importing it transforms', async () => {
    await expect(import('virtual:group-icons.css')).resolves.toBeDefined()
  })
})
