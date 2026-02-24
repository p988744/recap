describe('Authentication', () => {
  describe('Login page', () => {
    it('should display login form by default', async () => {
      // Wait for the app to load
      const loginUsername = await $('#login-username')
      await loginUsername.waitForDisplayed({ timeout: 15000 })

      const loginPassword = await $('#login-password')
      await expect(loginPassword).toBeDisplayed()
    })

    it('should show register tab', async () => {
      // Click the register tab
      const registerTab = await $('button*=註冊')
      await registerTab.click()

      const regUsername = await $('#reg-username')
      await regUsername.waitForDisplayed()

      const regName = await $('#reg-name')
      await expect(regName).toBeDisplayed()

      const regPassword = await $('#reg-password')
      await expect(regPassword).toBeDisplayed()
    })

    it('should show error on invalid login', async () => {
      // Switch back to login tab
      const loginTab = await $('button*=登入')
      await loginTab.click()

      const loginUsername = await $('#login-username')
      await loginUsername.waitForDisplayed()

      await loginUsername.setValue('invalid_user')

      const loginPassword = await $('#login-password')
      await loginPassword.setValue('wrong_password')

      // Submit the form
      const submitButton = await $('button[type="submit"]')
      await submitButton.click()

      // Wait for error message
      const errorMessage = await $('.border-red-200, .border-destructive')
      await errorMessage.waitForDisplayed({ timeout: 10000 })
    })
  })

  describe('Unauthenticated redirect', () => {
    it('should redirect to login when not authenticated', async () => {
      // The app should start on the login page if no token exists
      const loginUsername = await $('#login-username')
      await loginUsername.waitForDisplayed({ timeout: 15000 })
    })
  })
})
