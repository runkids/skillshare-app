import Button from '../../../components/Button';
import Spinner from '../../../components/Spinner';
import { useQuickActions } from './useQuickActions';

type Actions = ReturnType<typeof useQuickActions>;

function QuickActionForm({ actions }: { actions: Actions }) {
  const { context, progress, output, install, runAction, back } = actions;
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        void runAction();
      }}
      className="space-y-4"
    >
      <div className="rounded-[var(--r-ctl)] border border-[var(--line-2)] p-3">
        <p className="font-medium">Install {install?.name}?</p>
        <p className="text-sm text-pencil-light mt-1 break-all">{install?.source}</p>
        {install?.skill && (
          <p className="text-xs text-pencil-light">Selected skill: {install.skill}</p>
        )}
        {!install?.skill && (
          <p className="text-xs text-pencil-light mt-1">
            All skills discovered at this source will be installed.
          </p>
        )}
        <p className="text-xs mt-3">This adds the skill to the source directory shown above.</p>
      </div>
      <div className="flex gap-2">
        <Button type="submit" autoFocus loading={!!progress} disabled={!context || !!output}>
          Install
        </Button>
        <Button type="button" variant="secondary" disabled={!!progress} onClick={back}>
          Back
        </Button>
      </div>
    </form>
  );
}

function SkillSearch({ actions }: { actions: Actions }) {
  const {
    input,
    query,
    context,
    changeQuery,
    moveSelection,
    searching,
    error,
    results,
    selected,
    setSelected,
    setInstall,
    reviewSelected,
  } = actions;
  return (
    <>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          reviewSelected();
        }}
      >
        <input
          ref={input}
          aria-label="Search skills"
          placeholder="Search skills, or paste a GitHub URL or owner/repo…"
          className="ss-input w-full px-3 py-2 rounded-[var(--r-ctl)] border border-[var(--line-2)] bg-surface"
          value={query}
          disabled={!context}
          onChange={(event) => {
            changeQuery(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault();
              const delta = event.key === 'ArrowDown' ? 1 : -1;
              moveSelection(delta);
            }
          }}
        />
      </form>
      {searching && (
        <p role="status" className="flex items-center gap-2 text-sm mt-4">
          <Spinner size="sm" /> Searching…
        </p>
      )}
      {query.trim() && !searching && !error && results.length === 0 && (
        <p role="status" className="text-sm text-pencil-light mt-4">
          No matching skills.
        </p>
      )}
      <ul aria-label="Skill results" className="space-y-2 mt-4">
        {results.map((result, index) => (
          <li key={`${result.source}:${result.skill}:${result.name}`}>
            <button
              type="button"
              aria-pressed={selected === index}
              className={`w-full text-left p-3 rounded-[var(--r-ctl)] border ${selected === index ? 'border-pencil' : 'border-[var(--line-2)]'}`}
              onFocus={() => setSelected(index)}
              onClick={() => setInstall(result)}
            >
              <span className="block font-medium">{result.name}</span>
              <span className="block text-xs text-pencil-light break-all">{result.source}</span>
              <span className="block text-sm text-pencil-light mt-1">{result.description}</span>
            </button>
          </li>
        ))}
      </ul>
      <p className="text-xs text-pencil-light mt-4">
        ↑ ↓ Select · Enter Review installation · Esc Close
      </p>
    </>
  );
}

export default function QuickActionsPalette() {
  const actions = useQuickActions();
  const { context, install, progress, error, output, close } = actions;

  return (
    <main className="h-screen overflow-auto bg-surface p-5 text-pencil">
      <div className="flex items-center justify-between mb-3">
        <h1 className="text-lg font-semibold">Quick Actions</h1>
        <Button variant="ghost" size="sm" onClick={close}>
          Close · Esc
        </Button>
      </div>
      <p className="text-xs text-pencil-light mb-4 break-all">
        {context ? `${context.projectName} · ${context.sourceDir}` : 'Loading active project…'}
      </p>
      {install ? <QuickActionForm actions={actions} /> : <SkillSearch actions={actions} />}
      {progress && (
        <p role="status" className="mt-4 text-sm">
          {progress}
        </p>
      )}
      {error && (
        <p role="alert" className="mt-4 text-sm text-danger">
          {error}
        </p>
      )}
      {output && (
        <pre role="status" className="mt-4 text-xs whitespace-pre-wrap break-all">
          {output}
        </pre>
      )}
    </main>
  );
}
