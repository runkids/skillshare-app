/**
 * FirstRunWizard
 * Full-screen onboarding wizard shown when `.specforge/` doesn't exist.
 * Three steps: Welcome -> Choose Preset -> Initialize/Done.
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Anvil, ArrowRight, Check, Loader2, LayoutTemplate, FolderOpen } from 'lucide-react';
import { cn } from '../../lib/utils';

type WizardStep = 'welcome' | 'preset' | 'initializing';
type Preset = 'basic-sdd' | 'blank';

interface FirstRunWizardProps {
  projectDir: string;
  onComplete: () => void;
}

export function FirstRunWizard({ projectDir, onComplete }: FirstRunWizardProps) {
  const [step, setStep] = useState<WizardStep>('welcome');
  const [preset, setPreset] = useState<Preset>('basic-sdd');
  const [isInitializing, setIsInitializing] = useState(false);
  const [isDone, setIsDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleInitialize = useCallback(async () => {
    setStep('initializing');
    setIsInitializing(true);
    setError(null);

    try {
      await invoke('init_specforge_project', {
        projectDir,
        preset,
      });
      setIsDone(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsInitializing(false);
    }
  }, [projectDir, preset]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background">
      {/* Background pattern */}
      <div className="absolute inset-0 opacity-[0.08] pointer-events-none">
        <svg className="w-full h-full" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <pattern id="wizard-grid" width="40" height="40" patternUnits="userSpaceOnUse">
              <circle cx="20" cy="20" r="1" className="fill-muted-foreground" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#wizard-grid)" />
        </svg>
      </div>

      {/* Content */}
      <div className="relative z-10 flex flex-col items-center max-w-lg px-8">
        {step === 'welcome' && <WelcomeStep onNext={() => setStep('preset')} />}
        {step === 'preset' && (
          <PresetStep
            preset={preset}
            onPresetChange={setPreset}
            onNext={handleInitialize}
            onBack={() => setStep('welcome')}
          />
        )}
        {step === 'initializing' && (
          <InitializingStep
            isInitializing={isInitializing}
            isDone={isDone}
            error={error}
            onComplete={onComplete}
            onRetry={handleInitialize}
          />
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 1: Welcome
// ---------------------------------------------------------------------------

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="flex flex-col items-center text-center animate-in fade-in duration-300">
      {/* Logo / Icon */}
      <div className="w-20 h-20 rounded-2xl flex items-center justify-center bg-gradient-to-br from-blue-500/20 via-purple-500/15 to-cyan-500/10 border border-blue-500/20 shadow-lg shadow-blue-500/10 mb-8">
        <Anvil className="w-10 h-10 text-blue-400" />
      </div>

      <h1 className="text-2xl font-bold text-foreground mb-2">SpecForge</h1>
      <p className="text-base text-muted-foreground mb-2">
        The open platform for spec-driven development.
      </p>
      <p className="text-sm text-muted-foreground/70 mb-10">Forge your own spec workflow.</p>

      <button
        type="button"
        onClick={onNext}
        className={cn(
          'inline-flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-medium',
          'bg-blue-600/20 text-blue-400 border border-blue-500/30',
          'hover:bg-blue-600/30 hover:text-blue-300',
          'transition-colors duration-150',
          'focus:outline-none focus:ring-2 focus:ring-blue-500/40'
        )}
      >
        Get Started
        <ArrowRight className="w-4 h-4" />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 2: Choose Preset
// ---------------------------------------------------------------------------

function PresetStep({
  preset,
  onPresetChange,
  onNext,
  onBack,
}: {
  preset: Preset;
  onPresetChange: (p: Preset) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  return (
    <div className="flex flex-col items-center text-center animate-in fade-in duration-300 w-full">
      <h2 className="text-xl font-semibold text-foreground mb-2">Choose a starting point</h2>
      <p className="text-sm text-muted-foreground mb-8">
        You can always customize schemas and workflows later.
      </p>

      <div className="flex flex-col gap-3 w-full max-w-sm mb-8">
        <PresetCard
          selected={preset === 'basic-sdd'}
          onClick={() => onPresetChange('basic-sdd')}
          icon={LayoutTemplate}
          title="Basic SDD"
          description="Includes 3 schemas (spec, change-request, task) and a default workflow. Recommended for most projects."
          recommended
        />
        <PresetCard
          selected={preset === 'blank'}
          onClick={() => onPresetChange('blank')}
          icon={FolderOpen}
          title="Blank"
          description="Empty .specforge/ directory. Define your own schemas and workflows from scratch."
        />
      </div>

      <div className="flex items-center gap-3">
        <button
          type="button"
          onClick={onBack}
          className={cn(
            'px-4 py-2 rounded-lg text-sm font-medium',
            'text-muted-foreground hover:text-foreground',
            'transition-colors duration-150'
          )}
        >
          Back
        </button>
        <button
          type="button"
          onClick={onNext}
          className={cn(
            'inline-flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-medium',
            'bg-blue-600/20 text-blue-400 border border-blue-500/30',
            'hover:bg-blue-600/30 hover:text-blue-300',
            'transition-colors duration-150',
            'focus:outline-none focus:ring-2 focus:ring-blue-500/40'
          )}
        >
          Initialize
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}

function PresetCard({
  selected,
  onClick,
  icon: Icon,
  title,
  description,
  recommended,
}: {
  selected: boolean;
  onClick: () => void;
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  recommended?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-full text-left p-4 rounded-lg border transition-all duration-150',
        'focus:outline-none focus:ring-2 focus:ring-blue-500/40',
        selected
          ? 'border-blue-500/50 bg-blue-500/10'
          : 'border-border bg-card hover:border-border/80 hover:bg-accent/30'
      )}
    >
      <div className="flex items-start gap-3">
        <div
          className={cn(
            'w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5',
            selected
              ? 'bg-blue-500/20 text-blue-400'
              : 'bg-muted text-muted-foreground'
          )}
        >
          <Icon className="w-4.5 h-4.5" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-foreground">{title}</span>
            {recommended && (
              <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-500/20 text-blue-400 border border-blue-500/30">
                Recommended
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground mt-1 leading-relaxed">{description}</p>
        </div>
        <div
          className={cn(
            'w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0 mt-1',
            selected ? 'border-blue-500 bg-blue-500' : 'border-muted-foreground/30'
          )}
        >
          {selected && <div className="w-1.5 h-1.5 rounded-full bg-white" />}
        </div>
      </div>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Step 3: Initializing / Done
// ---------------------------------------------------------------------------

function InitializingStep({
  isInitializing,
  isDone,
  error,
  onComplete,
  onRetry,
}: {
  isInitializing: boolean;
  isDone: boolean;
  error: string | null;
  onComplete: () => void;
  onRetry: () => void;
}) {
  if (error) {
    return (
      <div className="flex flex-col items-center text-center animate-in fade-in duration-300">
        <div className="w-16 h-16 rounded-2xl flex items-center justify-center bg-red-500/20 border border-red-500/30 mb-6">
          <span className="text-2xl text-red-400">!</span>
        </div>
        <h2 className="text-xl font-semibold text-foreground mb-2">Initialization Failed</h2>
        <p className="text-sm text-muted-foreground mb-2">{error}</p>
        <button
          type="button"
          onClick={onRetry}
          className={cn(
            'mt-4 inline-flex items-center gap-2 px-5 py-2 rounded-lg text-sm font-medium',
            'bg-blue-600/20 text-blue-400 border border-blue-500/30',
            'hover:bg-blue-600/30 hover:text-blue-300',
            'transition-colors duration-150'
          )}
        >
          Retry
        </button>
      </div>
    );
  }

  if (isInitializing) {
    return (
      <div className="flex flex-col items-center text-center animate-in fade-in duration-300">
        <Loader2 className="w-10 h-10 text-blue-400 animate-spin mb-6" />
        <h2 className="text-lg font-medium text-foreground">Initializing...</h2>
        <p className="text-sm text-muted-foreground mt-1">Setting up your .specforge/ directory</p>
      </div>
    );
  }

  if (isDone) {
    return (
      <div className="flex flex-col items-center text-center animate-in fade-in duration-300">
        <div className="w-16 h-16 rounded-2xl flex items-center justify-center bg-green-500/20 border border-green-500/30 shadow-lg shadow-green-500/10 mb-6">
          <Check className="w-8 h-8 text-green-400" />
        </div>
        <h2 className="text-xl font-semibold text-foreground mb-2">SpecForge Initialized!</h2>
        <p className="text-sm text-muted-foreground mb-8">
          Your .specforge/ directory is ready. Start creating specs.
        </p>
        <button
          type="button"
          onClick={onComplete}
          className={cn(
            'inline-flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-medium',
            'bg-green-600/20 text-green-400 border border-green-500/30',
            'hover:bg-green-600/30 hover:text-green-300',
            'transition-colors duration-150',
            'focus:outline-none focus:ring-2 focus:ring-green-500/40'
          )}
        >
          Start Building
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>
    );
  }

  return null;
}
