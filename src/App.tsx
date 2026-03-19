import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/queryClient';
import { ToastProvider } from './components/Toast';
import { ThemeProvider } from './context/ThemeContext';
import { TauriProvider, useTauri } from './desktop/context/TauriContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import TitleBar from './desktop/components/TitleBar';
import CliWebView from './desktop/components/CliWebView';
import OnboardingPage from './desktop/pages/OnboardingPage';
import ProjectsPage from './desktop/pages/ProjectsPage';

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

function ConditionalTitleBar() {
  const location = useLocation();
  if (location.pathname === '/onboarding') return null;
  return <TitleBar />;
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <ToastProvider>
          <TauriProvider>
            <BrowserRouter>
              <ErrorBoundary>
                <div className="h-screen flex flex-col">
                  <ConditionalTitleBar />
                  <OnboardingGuard>
                    <Routes>
                      <Route path="/onboarding" element={<OnboardingPage />} />
                      <Route path="/projects" element={<ProjectsPage />} />
                      <Route path="/*" element={<CliWebView />} />
                    </Routes>
                  </OnboardingGuard>
                </div>
              </ErrorBoundary>
            </BrowserRouter>
          </TauriProvider>
        </ToastProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}
