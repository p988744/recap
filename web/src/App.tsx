import { BrowserRouter, Routes, Route } from 'react-router-dom'
import { AuthProvider } from '@/lib/auth'
import { ProtectedRoute } from '@/components/ProtectedRoute'
import { Layout } from '@/components/Layout'
import { LoginPage } from '@/pages/Login'
import { OnboardingPage } from '@/pages/Onboarding'
import { ThisWeekPage, DayDetailPage, ProjectDayDetailPage } from '@/pages/ThisWeek'
import { ProjectsPage, TimelinePeriodDetailPage } from '@/pages/Projects'
import { SettingsPage } from '@/pages/Settings'
import { QuotaPage } from '@/pages/Quota'

function App() {
  return (
    <AuthProvider>
      <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <Routes>
          {/* Public routes */}
          <Route path="/login" element={<LoginPage />} />
          <Route path="/onboarding" element={<OnboardingPage />} />

          {/* Protected routes */}
          <Route
            path="/"
            element={
              <ProtectedRoute>
                <Layout />
              </ProtectedRoute>
            }
          >
            <Route index element={<ThisWeekPage />} />
            <Route path="day/:date" element={<DayDetailPage />} />
            <Route path="day/:date/:projectPath" element={<ProjectDayDetailPage />} />
            <Route path="projects" element={<ProjectsPage />} />
            <Route path="projects/:projectName/period" element={<TimelinePeriodDetailPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="quota" element={<QuotaPage />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  )
}

export default App
