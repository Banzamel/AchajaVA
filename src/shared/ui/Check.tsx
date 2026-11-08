// A checkbox with a rounded box and its label on the right.
interface Props {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Check({ label, checked, onChange, disabled }: Props) {
  return (
    <label className={`check ${disabled ? 'disabled' : ''}`}>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      <span className="check-box" aria-hidden="true" />
      <span>{label}</span>
    </label>
  );
}
