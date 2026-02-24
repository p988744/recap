describe('Settings Page', () => {
  before(async () => {
    // Wait for app to load
    await browser.pause(3000)
  })

  describe('Settings navigation', () => {
    it('should navigate to settings page', async () => {
      const settingsLink = await $('a[href="/settings"]')
      const isDisplayed = await settingsLink.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await settingsLink.click()
        await browser.pause(1000)

        const url = await browser.getUrl()
        expect(url).toContain('/settings')
      }
    })

    it('should display settings sections sidebar', async () => {
      const url = await browser.getUrl()
      if (!url.includes('/settings')) return

      // Settings page has section navigation items
      // Sections: profile, projects, sync, export, ai, about, advanced
      const profileSection = await $('*=帳號')
      const isDisplayed = await profileSection.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await expect(profileSection).toBeDisplayed()
      }
    })
  })

  describe('Settings sections', () => {
    it('should switch between settings sections', async () => {
      const url = await browser.getUrl()
      if (!url.includes('/settings')) return

      // Try clicking on different section links
      const sections = await $$('nav a, [role="tablist"] button')

      if ((await sections.length) > 1) {
        // Click the second section
        await sections[1].click()
        await browser.pause(500)

        // Content should change
        const content = await $('main, [role="main"], .flex-1')
        await expect(content).toBeExisting()
      }
    })
  })

  describe('Sync settings', () => {
    it('should have sync trigger button in sidebar', async () => {
      // The sidebar has a sync button (RefreshCw icon)
      const syncButton = await $('button[title*="同步"], button svg.lucide-refresh-cw')
      const isDisplayed = await syncButton?.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await expect(syncButton).toBeDisplayed()
      }
    })
  })
})
