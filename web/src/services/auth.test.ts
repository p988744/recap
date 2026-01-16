import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import { mockUser, mockAppStatus, mockTokenResponse } from '@/test/fixtures'
import * as auth from './auth'

describe('auth service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.clear()
  })

  describe('getAppStatus', () => {
    it('should return app status', async () => {
      mockCommandValue('get_app_status', mockAppStatus)

      const result = await auth.getAppStatus()

      expect(result).toEqual(mockAppStatus)
      expect(mockInvoke).toHaveBeenCalledWith('get_app_status', undefined)
    })

    it('should throw on error', async () => {
      mockCommandError('get_app_status', 'Database error')

      await expect(auth.getAppStatus()).rejects.toThrow('Database error')
    })
  })

  describe('register', () => {
    it('should register a new user', async () => {
      mockCommandValue('register_user', mockUser)

      const request = {
        username: 'newuser',
        password: 'password123',
        name: 'New User',
        email: 'new@example.com',
      }
      const result = await auth.register(request)

      expect(result).toEqual(mockUser)
      expect(mockInvoke).toHaveBeenCalledWith('register_user', { request })
    })

    it('should throw on duplicate username', async () => {
      mockCommandError('register_user', 'Username already exists')

      const request = {
        username: 'existing',
        password: 'password',
        name: 'Test',
      }

      await expect(auth.register(request)).rejects.toThrow('Username already exists')
    })
  })

  describe('login', () => {
    it('should return token on successful login', async () => {
      mockCommandValue('login', mockTokenResponse)

      const request = { username: 'testuser', password: 'password' }
      const result = await auth.login(request)

      expect(result).toEqual(mockTokenResponse)
      expect(result.access_token).toBe('mock-jwt-token-12345')
    })

    it('should throw on invalid credentials', async () => {
      mockCommandError('login', 'Invalid credentials')

      const request = { username: 'wrong', password: 'wrong' }

      await expect(auth.login(request)).rejects.toThrow('Invalid credentials')
    })
  })

  describe('autoLogin', () => {
    it('should return token for local mode auto-login', async () => {
      mockCommandValue('auto_login', mockTokenResponse)

      const result = await auth.autoLogin()

      expect(result).toEqual(mockTokenResponse)
      expect(mockInvoke).toHaveBeenCalledWith('auto_login', undefined)
    })
  })

  describe('getCurrentUser', () => {
    it('should get user with provided token', async () => {
      mockCommandValue('get_current_user', mockUser)

      const result = await auth.getCurrentUser('custom-token')

      expect(result).toEqual(mockUser)
      expect(mockInvoke).toHaveBeenCalledWith('get_current_user', { token: 'custom-token' })
    })

    it('should get user with stored token when no token provided', async () => {
      localStorage.setItem('recap_auth_token', 'stored-token')
      mockCommandValue('get_current_user', mockUser)

      const result = await auth.getCurrentUser()

      expect(result).toEqual(mockUser)
      expect(mockInvoke).toHaveBeenCalledWith('get_current_user', { token: 'stored-token' })
    })

    it('should redirect to login when no token available', async () => {
      // No token in localStorage and no token provided
      await expect(auth.getCurrentUser()).rejects.toThrow('No auth token')
      expect(window.location.href).toBe('/login')
    })
  })

  describe('getProfile', () => {
    it('should get user profile with stored token', async () => {
      localStorage.setItem('recap_auth_token', 'stored-token')
      mockCommandValue('get_profile', mockUser)

      const result = await auth.getProfile()

      expect(result).toEqual(mockUser)
    })
  })

  describe('updateProfile', () => {
    it('should update user profile', async () => {
      localStorage.setItem('recap_auth_token', 'stored-token')
      const updatedUser = { ...mockUser, name: 'Updated Name' }
      mockCommandValue('update_profile', updatedUser)

      const result = await auth.updateProfile({ name: 'Updated Name' })

      expect(result.name).toBe('Updated Name')
    })
  })
})
