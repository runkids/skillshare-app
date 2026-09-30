import { useState, useCallback, useEffect, useReducer, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import WelcomeStep from '../components/OnboardingSteps/WelcomeStep';
import ProjectSetupStep from '../components/OnboardingSteps/ProjectSetupStep';
import FirstSyncStep from '../components/OnboardingSteps/FirstSyncStep';
import OnboardingStepper from '../components/OnboardingSteps/OnboardingStepper';
import { flowReducer, initialFlow } from '../components/OnboardingSteps/onboarding-flow';
import '../components/OnboardingSteps/onboarding.css';
import { tauriBridge } from '../api/tauri-bridge';
import { useTauri } from '../context/TauriContext';
import { useProjects } from '../context/ProjectContext';

const LEAVE_MS = 240;
const CAPTIONS = ['Install or locate', 'Pick targets', 'Syncing your skills'];

export default function OnboardingPage() {
  const [flow, dispatch] = useReducer(flowReducer, initialFlow);
  // The step on screen lags `flow.step` so the outgoing content can animate away first.
  const [shown, setShown] = useState(flow.step);
  const leaveTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const navigate = useNavigate();
  const { refresh } = useTauri();
  const { refresh: refreshProjects } = useProjects();
  const { cliPath } = flow;
  const leaving = flow.step !== shown;

  useEffect(() => {
    if (flow.step === shown) return;
    leaveTimer.current = setTimeout(() => setShown(flow.step), LEAVE_MS);
    return () => clearTimeout(leaveTimer.current);
  }, [flow.step, shown]);

  const handleWelcomeComplete = useCallback(
    (path: string) => dispatch({ type: 'cli-ready', cliPath: path }),
    []
  );
  const handleProjectComplete = useCallback(() => dispatch({ type: 'init-done' }), []);
  const handleSynced = useCallback(() => dispatch({ type: 'sync-done' }), []);

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
    <div className="ob-root flex min-h-screen items-center justify-center bg-bg p-8">
      <div className="flex h-[580px] w-[960px] max-w-full overflow-hidden rounded-xl border border-line bg-surface">
        <aside className="flex w-[290px] flex-none flex-col gap-9 border-r border-line-soft bg-side px-7 py-8">
          <div>
            <div className="text-[17px] font-bold leading-none tracking-[-0.01em] text-ink">
              skillshare
            </div>
            <div className="mt-1.5 text-[13px] text-ink-2">
              One folder of skills, synced to every AI tool.
            </div>
          </div>
          <OnboardingStepper flow={flow} caption={CAPTIONS[flow.step]} />
          <div className="mt-auto text-xs text-ink-2">
            About a minute. You can change everything later in Settings.
          </div>
        </aside>
        <section
          className={`flex min-w-0 flex-1 flex-col px-11 py-9 ${leaving ? 'ob-leave' : ''}`}
          aria-live="polite"
        >
          {shown === 0 && <WelcomeStep onComplete={handleWelcomeComplete} />}
          {shown === 1 && cliPath && (
            <ProjectSetupStep cliPath={cliPath} onComplete={handleProjectComplete} />
          )}
          {shown === 2 && cliPath && (
            <FirstSyncStep
              cliPath={cliPath}
              onComplete={handleSyncComplete}
              onSynced={handleSynced}
            />
          )}
        </section>
      </div>
    </div>
  );
}
