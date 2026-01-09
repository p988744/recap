import { Navigate, useLocation } from 'react-router-dom'
import { useAuth } from '@/lib/auth'

interface ProtectedRouteProps {
  children: React.ReactNode
}

export function ProtectedRoute({ children }: ProtectedRouteProps) {
  const { isAuthenticated, isLoading, needsOnboarding } = useAuth()
  const location = useLocation()

  if (isLoading) {
    return (
      <div className="min-h-screen bg-[#F5F2E8] flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <div className="w-14 h-14 rounded-xl bg-[#1F1D1A] flex flex-col items-center justify-center animate-pulse p-2">
            <span className="text-[#F9F7F2] text-sm font-medium tracking-tight">Recap</span>
            <div className="w-8 h-0.5 bg-[#B09872] mt-0.5 rounded-full opacity-70" />
          </div>
          <p className="text-[#3D2832]/70">Loading...</p>
        </div>
      </div>
    )
  }

  // If no users yet, redirect to onboarding
  if (needsOnboarding) {
    return <Navigate to="/onboarding" replace />
  }

  if (!isAuthenticated) {
    // Redirect to login page but save the current location
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  return <>{children}</>
}
