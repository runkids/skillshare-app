import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/queryClient';
import { ToastProvider } from './components/Toast';
import { ThemeProvider } from './context/ThemeContext';
import { TauriProvider, useTauri } from './desktop/context/TauriContext';
import { ProjectProvider } from './desktop/context/ProjectContext';
import { TerminalProvider } from './desktop/context/TerminalContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import MainView from './desktop/components/MainView';
import OnboardingPage from './desktop/pages/OnboardingPage';
import SettingsPage from './desktop/pages/SettingsPage';

function OnboardingGuard({ children }: { children: React.ReactNode }) {
  const { appInfo, loading } = useTauri();
  const location = useLocation();
  if (loading) return null;
  if (!appInfo) return <>{children}</>;
  if (!appInfo.onboarding.completed && location.pathname !== '/onboarding') {
    return <Navigate to="/onboarding" replace />;
  }
  return <>{children}</>;
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <ToastProvider>
          <TauriProvider>
            <ProjectProvider>
              <TerminalProvider>
                <BrowserRouter>
                  <ErrorBoundary>
                    <div className="h-screen flex flex-col">
                      <OnboardingGuard>
                        <Routes>
                          <Route path="/onboarding" element={<OnboardingPage />} />
                          <Route path="/settings" element={<SettingsPage />} />
                          <Route path="/*" element={<MainView />} />
                        </Routes>
                      </OnboardingGuard>
                    </div>
                  </ErrorBoundary>
                </BrowserRouter>
              </TerminalProvider>
            </ProjectProvider>
          </TauriProvider>
        </ToastProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}
