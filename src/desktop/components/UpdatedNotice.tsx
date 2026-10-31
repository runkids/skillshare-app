import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Check, X } from 'lucide-react';
import Button from '../../components/Button';
import { JUST_UPDATED_KEY, type JustUpdated } from '../hooks/useAppUpdate';
import { releaseNoteItems } from '../utils/release-notes';

const APP_REPO = 'https://github.com/runkids/skillshare-app';

function readJustUpdated(): JustUpdated | null {
  try {
    const raw = localStorage.getItem(JUST_UPDATED_KEY);
    return raw ? (JSON.parse(raw) as JustUpdated) : null;
  } catch {
    return null;
  }
}

function forget() {
  try {
    localStorage.removeItem(JUST_UPDATED_KEY);
  } catch {
    // Nothing to clean up when storage is unavailable.
  }
}

/** After restarting into an update, say once which version is running and what changed. */
export default function UpdatedNotice() {
  const [note, setNote] = useState<JustUpdated | null>(null);

  useEffect(() => {
    const pending = readJustUpdated();
    if (!pending) return;
    getVersion()
      .then((running) => {
        // Only a restart into the new version counts; otherwise the update did not apply.
        if (running.replace(/^v/, '') === pending.version.replace(/^v/, '')) setNote(pending);
        else forget();
      })
      .catch(() => {});
  }, []);

  if (!note) return null;
  const version = `v${note.version.replace(/^v/, '')}`;
  const items = releaseNoteItems(note.notes).slice(0, 5);
  const close = () => {
    forget();
    setNote(null);
  };

  return (
    <section
      role="status"
      aria-label="Update finished"
      className="absolute top-4 right-4 z-40 w-[380px] flex flex-col gap-3 p-[18px] rounded-[var(--r-box)] bg-[var(--surface)] shadow-[var(--sh-float)]"
      style={{ border: 'var(--frame)' }}
    >
      <div className="flex items-start gap-3">
        <span className="w-8 h-8 shrink-0 flex items-center justify-center rounded-[var(--r-ctl)] bg-[var(--ok-bg)] text-[var(--ok)]">
          <Check size={18} strokeWidth={2.25} />
        </span>
        <div className="flex-1 min-w-0 flex flex-col gap-0.5">
          <span style={{ font: 'var(--h2)', letterSpacing: 'var(--h2-track)' }}>
            Updated to {version}
          </span>
          <span className="text-[13px] text-[var(--ink-2)]">
            skillshare App is on the new version.
          </span>
        </div>
        <button
          type="button"
          aria-label="Close"
          onClick={close}
          className="w-[26px] h-[26px] shrink-0 flex items-center justify-center rounded-[var(--r-ctl)] text-[var(--ink-3)] hover:bg-[var(--sel)]"
        >
          <X size={14} />
        </button>
      </div>
      {items.length > 0 && (
        <div className="flex flex-col gap-1.5 pt-3" style={{ borderTop: 'var(--sep)' }}>
          <span className="text-[13px] font-semibold">What’s new</span>
          <ul className="m-0 pl-[18px] text-[13px] text-[var(--ink-2)] flex flex-col gap-[3px]">
            {items.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      )}
      <div className="flex items-center justify-between gap-3">
        <a
          href={`${APP_REPO}/releases/tag/${version}`}
          onClick={(e) => {
            e.preventDefault();
            void openUrl(`${APP_REPO}/releases/tag/${version}`);
          }}
          className="text-[13px] text-[var(--accent)] hover:underline"
        >
          Full release notes
        </a>
        <Button size="sm" onClick={close}>
          Got it
        </Button>
      </div>
    </section>
  );
}
