describe('Dashboard (This Week)', () => {
  before(async () => {
    // Wait for the app to fully load
    await browser.pause(3000)
  })

  describe('Page load', () => {
    it('should display the this week page header when authenticated', async () => {
      const header = await $('*=本週工作')
      await header.waitForDisplayed({ timeout: 10000 })
      await expect(header).toBeDisplayed()
    })

    it('should display week navigation controls', async () => {
      const header = await $('*=本週工作')
      await expect(header).toBeDisplayed()

      // Should have some date range or week navigation
      // Week header typically shows date range
      const weekHeader = await $('[class*="week"], [class*="Week"]')
      await weekHeader.waitForDisplayed({ timeout: 5000 })
      await expect(weekHeader).toBeDisplayed()
    })
  })

  describe('Date navigation', () => {
    it('should have clickable date elements', async () => {
      const header = await $('*=本週工作')
      await expect(header).toBeDisplayed()

      // Look for any navigation buttons (arrows for week switching)
      const navButtons = await $$('button svg')
      expect(navButtons.length).toBeGreaterThan(0)
    })
  })

  describe('Content sections', () => {
    it('should display work content area', async () => {
      const header = await $('*=本週工作')
      await expect(header).toBeDisplayed()

      // Main content area should exist
      const mainContent = await $('main, [role="main"], .flex-1')
      await expect(mainContent).toBeExisting()
    })
  })
})
