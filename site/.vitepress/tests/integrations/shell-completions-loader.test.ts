import loader from '../../lib/integrations/shell-completions.data'

const shells = await loader.load([])

describe('the shell completions loader', () => {
  it('adds the PowerShell line to the profile and leaves what the profile holds', () => {
    const code = shells.find(({ slug }) => slug === 'powershell')
      ?.codeHtml.replaceAll(/<[^>]+>/g, '')
    expect(code).toContain('if (!(Test-Path -Path $PROFILE.CurrentUserAllHosts)) {')
    expect(code).toContain('Add-Content -Path $PROFILE.CurrentUserAllHosts -Value')
    expect(code).not.toMatch(/>\s*\$PROFILE/)
  })
})
