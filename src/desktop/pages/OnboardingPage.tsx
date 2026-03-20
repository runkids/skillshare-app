import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import WelcomeStep from '../components/OnboardingSteps/WelcomeStep';
import ProjectSetupStep from '../components/OnboardingSteps/ProjectSetupStep';
import FirstSyncStep from '../components/OnboardingSteps/FirstSyncStep';
import { tauriBridge } from '../api/tauri-bridge';
import { useTauri } from '../context/TauriContext';
import { useProjects } from '../context/ProjectContext';

const STEPS = ['CLI Setup', 'Init', 'Sync'] as const;

export default function OnboardingPage() {
  const [step, setStep] = useState(0);
  const [cliPath, setCliPath] = useState<string | null>(null);
  const navigate = useNavigate();
  const { refresh } = useTauri();
  const { refresh: refreshProjects } = useProjects();

  const handleWelcomeComplete = useCallback((path: string) => {
    setCliPath(path);
    setStep(1);
  }, []);

  const handleProjectComplete = useCallback(() => {
    setStep(2);
  }, []);

  const handleSyncComplete = useCallback(async () => {
    // Start the server before navigating to the main app
    if (cliPath) {
      try {
        await tauriBridge.startServer(cliPath);
      } catch {
        // Server start failure is non-fatal for onboarding
      }
    }
    await Promise.all([refresh(), refreshProjects()]);
    navigate('/', { replace: true });
  }, [cliPath, navigate, refresh, refreshProjects]);

  return (
    <div className="min-h-screen bg-paper flex flex-col items-center justify-center p-8">
      {/* Step indicator */}
      <div className="flex items-center gap-2 mb-12">
        {STEPS.map((label, i) => (
          <div key={label} className="flex items-center gap-2">
            <div
              className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors ${
                i <= step ? 'bg-pencil text-paper' : 'bg-muted text-muted-dark'
              }`}
            >
              {i + 1}
            </div>
            <span
              className={`text-sm hidden sm:inline ${
                i <= step ? 'text-pencil font-medium' : 'text-muted-dark'
              }`}
            >
              {label}
            </span>
            {i < STEPS.length - 1 && (
              <div className={`w-12 h-0.5 mx-1 ${i < step ? 'bg-pencil' : 'bg-muted'}`} />
            )}
          </div>
        ))}
      </div>

      {/* Step content */}
      <div className="w-full max-w-lg animate-fade-in">
        {step === 0 && <WelcomeStep onComplete={handleWelcomeComplete} />}
        {step === 1 && cliPath && (
          <ProjectSetupStep cliPath={cliPath} onComplete={handleProjectComplete} />
        )}
        {step === 2 && cliPath && (
          <FirstSyncStep cliPath={cliPath} onComplete={handleSyncComplete} />
        )}
      </div>
    </div>
  );
}
