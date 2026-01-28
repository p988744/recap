import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useProfileForm } from './useProfileForm'
import { auth } from '@/services'

vi.mock('@/services', () => ({
  auth: {
    updateProfile: vi.fn(),
  },
}))

describe('useProfileForm', () => {
  const mockUser = {
    id: 'user-1',
    name: 'Test User',
    email: 'test@example.com',
    title: 'Engineer',
    employee_id: 'EMP001',
    department_id: 'DEP001',
    is_active: true,
    is_admin: false,
    created_at: '2024-01-01T00:00:00Z',
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with empty values when user is null', () => {
    const { result } = renderHook(() => useProfileForm(null))

    expect(result.current.profileName).toBe('')
    expect(result.current.profileEmail).toBe('')
    expect(result.current.profileTitle).toBe('')
    expect(result.current.profileEmployeeId).toBe('')
    expect(result.current.profileDepartment).toBe('')
  })

  it('should initialize with user values when user is provided', () => {
    const { result } = renderHook(() => useProfileForm(mockUser))

    expect(result.current.profileName).toBe('Test User')
    expect(result.current.profileEmail).toBe('test@example.com')
    expect(result.current.profileTitle).toBe('Engineer')
    expect(result.current.profileEmployeeId).toBe('EMP001')
    expect(result.current.profileDepartment).toBe('DEP001')
  })

  it('should update profile name', () => {
    const { result } = renderHook(() => useProfileForm(mockUser))

    act(() => {
      result.current.setProfileName('New Name')
    })

    expect(result.current.profileName).toBe('New Name')
  })

  it('should call auth.updateProfile on save', async () => {
    vi.mocked(auth.updateProfile).mockResolvedValue(mockUser)
    const setMessage = vi.fn()
    const { result } = renderHook(() => useProfileForm(mockUser))

    await act(async () => {
      await result.current.handleSave(setMessage)
    })

    expect(auth.updateProfile).toHaveBeenCalledWith({
      name: 'Test User',
      email: 'test@example.com',
      title: 'Engineer',
      employee_id: 'EMP001',
      department_id: 'DEP001',
    })
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '個人資料已更新' })
  })

  it('should handle save error', async () => {
    vi.mocked(auth.updateProfile).mockRejectedValue(new Error('Update failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useProfileForm(mockUser))

    await act(async () => {
      await result.current.handleSave(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Update failed' })
  })

  it('should set saving state during save', async () => {
    let resolvePromise: () => void
    vi.mocked(auth.updateProfile).mockImplementation(
      () => new Promise(resolve => { resolvePromise = () => resolve(mockUser) })
    )
    const setMessage = vi.fn()
    const { result } = renderHook(() => useProfileForm(mockUser))

    expect(result.current.saving).toBe(false)

    let savePromise: Promise<void>
    act(() => {
      savePromise = result.current.handleSave(setMessage)
    })

    expect(result.current.saving).toBe(true)

    await act(async () => {
      resolvePromise!()
      await savePromise
    })

    expect(result.current.saving).toBe(false)
  })
})
