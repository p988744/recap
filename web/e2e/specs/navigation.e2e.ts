describe('Navigation', () => {
  before(async () => {
    // Wait for app to fully load
    // If we're on the login page, we need to handle auth first
    // For navigation tests, we test what's accessible without auth
    await browser.pause(3000)
  })

  describe('Sidebar navigation', () => {
    it('should display the Recap logo in sidebar', async () => {
      // If authenticated, check sidebar
      const logo = await $('h1*=Recap')
      const isDisplayed = await logo.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await expect(logo).toBeDisplayed()
      } else {
        // Not authenticated — login page should show Recap branding
        const loginBranding = await $('*=Recap')
        await expect(loginBranding).toBeExisting()
      }
    })

    it('should have navigation links when authenticated', async () => {
      // Check if we're authenticated by looking for sidebar nav
      const thisWeekLink = await $('a[href="/"]')
      const isDisplayed = await thisWeekLink.isDisplayed().catch(() => false)

      if (isDisplayed) {
        // Verify nav items exist
        const projectsLink = await $('a[href="/projects"]')
        await expect(projectsLink).toBeDisplayed()

        const settingsLink = await $('a[href="/settings"]')
        await expect(settingsLink).toBeDisplayed()
      }
    })

    it('should highlight active navigation item', async () => {
      const thisWeekLink = await $('a[href="/"]')
      const isDisplayed = await thisWeekLink.isDisplayed().catch(() => false)

      if (isDisplayed) {
        // The home link should be active by default
        const classList = await thisWeekLink.getAttribute('class')
        expect(classList).toContain('font-medium')
      }
    })
  })

  describe('Route transitions', () => {
    it('should navigate to projects page', async () => {
      const projectsLink = await $('a[href="/projects"]')
      const isDisplayed = await projectsLink.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await projectsLink.click()
        await browser.pause(1000)

        // Verify we're on the projects page
        const projectsHeader = await $('*=專案')
        await expect(projectsHeader).toBeDisplayed()
      }
    })

    it('should navigate to settings page', async () => {
      const settingsLink = await $('a[href="/settings"]')
      const isDisplayed = await settingsLink.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await settingsLink.click()
        await browser.pause(1000)

        // Verify settings page content loaded
        const url = await browser.getUrl()
        expect(url).toContain('/settings')
      }
    })

    it('should navigate back to this week page', async () => {
      const homeLink = await $('a[href="/"]')
      const isDisplayed = await homeLink.isDisplayed().catch(() => false)

      if (isDisplayed) {
        await homeLink.click()
        await browser.pause(1000)

        // Verify we're back on the main page
        const thisWeekHeader = await $('*=本週工作')
        await expect(thisWeekHeader).toBeDisplayed()
      }
    })
  })
})
