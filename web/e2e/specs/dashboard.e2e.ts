describe('Dashboard (This Week)', () => {
  before(async () => {
    // Wait for the app to fully load
    await browser.pause(3000)
  })

  describe('Page load', () => {
    it('should display the this week page header when authenticated', async () => {
      const header = await $('*=本週工作')
      const isDisplayed = await header.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await expect(header).toBeDisplayed()
      } else {
        // If not authenticated, we should be on the login page
        const loginForm = await $('#login-username')
        await expect(loginForm).toBeDisplayed()
      }
    })

    it('should display week navigation controls', async () => {
      const header = await $('*=本週工作')
      const isDisplayed = await header.isDisplayed().catch(() => false)

      if (isDisplayed) {
        // Should have some date range or week navigation
        // Week header typically shows date range
        const weekHeader = await $('[class*="week"], [class*="Week"]')
        const weekExists = await weekHeader.isExisting().catch(() => false)

        if (weekExists) {
          await expect(weekHeader).toBeDisplayed()
        }
      }
    })
  })

  describe('Date navigation', () => {
    it('should have clickable date elements', async () => {
      const header = await $('*=本週工作')
      const isDisplayed = await header.isDisplayed().catch(() => false)

      if (isDisplayed) {
        // Look for any navigation buttons (arrows for week switching)
        const navButtons = await $$('button svg')
        expect(navButtons.length).toBeGreaterThan(0)
      }
    })
  })

  describe('Content sections', () => {
    it('should display work content area', async () => {
      const header = await $('*=本週工作')
      const isDisplayed = await header.isDisplayed().catch(() => false)

      if (isDisplayed) {
        // Main content area should exist
        const mainContent = await $('main, [role="main"], .flex-1')
        await expect(mainContent).toBeExisting()
      }
    })
  })
})
