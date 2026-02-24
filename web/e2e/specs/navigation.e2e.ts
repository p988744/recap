describe('Navigation', () => {
  before(async () => {
    // Wait for app to fully load
    await browser.pause(3000)
  })

  describe('Sidebar navigation', () => {
    it('should display the Recap logo in sidebar', async () => {
      const logo = await $('h1*=Recap')
      await logo.waitForDisplayed({ timeout: 10000 })
      await expect(logo).toBeDisplayed()
    })

    it('should have navigation links when authenticated', async () => {
      const thisWeekLink = await $('a[href="/"]')
      await thisWeekLink.waitForDisplayed({ timeout: 5000 })

      // Verify nav items exist
      const projectsLink = await $('a[href="/projects"]')
      await expect(projectsLink).toBeDisplayed()

      const settingsLink = await $('a[href="/settings"]')
      await expect(settingsLink).toBeDisplayed()
    })

    it('should highlight active navigation item', async () => {
      const thisWeekLink = await $('a[href="/"]')
      await thisWeekLink.waitForDisplayed({ timeout: 5000 })

      // The home link should be active by default
      const classList = await thisWeekLink.getAttribute('class')
      expect(classList).toContain('font-medium')
    })
  })

  describe('Route transitions', () => {
    it('should navigate to projects page', async () => {
      const projectsLink = await $('a[href="/projects"]')
      await projectsLink.waitForDisplayed({ timeout: 5000 })

      await projectsLink.click()
      await browser.pause(1000)

      // Verify we're on the projects page
      const projectsHeader = await $('*=專案')
      await expect(projectsHeader).toBeDisplayed()
    })

    it('should navigate to settings page', async () => {
      const settingsLink = await $('a[href="/settings"]')
      await settingsLink.waitForDisplayed({ timeout: 5000 })

      await settingsLink.click()
      await browser.pause(1000)

      // Verify settings page content loaded
      const url = await browser.getUrl()
      expect(url).toContain('/settings')
    })

    it('should navigate back to this week page', async () => {
      const homeLink = await $('a[href="/"]')
      await homeLink.waitForDisplayed({ timeout: 5000 })

      await homeLink.click()
      await browser.pause(1000)

      // Verify we're back on the main page
      const thisWeekHeader = await $('*=本週工作')
      await expect(thisWeekHeader).toBeDisplayed()
    })
  })
})
