interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Id of the element that names the setting. */
  labelledBy: string;
  describedBy?: string;
  disabled?: boolean;
}

/** On/off setting toggle; mirrors the skillshare web UI's `.ss-sw`. */
export default function Switch({
  checked,
  onChange,
  labelledBy,
  describedBy,
  disabled,
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`ss-sw ${checked ? 'on' : ''}`}
    >
      <i />
    </button>
  );
}
