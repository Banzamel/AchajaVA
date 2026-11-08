// A multi-line field in a rounded frame (looks like the single-line fields).
interface Props {
  label?: string;
  value: string;
  rows: number;
  onChange: (value: string) => void;
}

export function TextArea({ label, value, rows, onChange }: Props) {
  return (
    <label className="field">
      {label ? <span className="field-label">{label}</span> : null}
      <textarea className="input textarea" rows={rows} value={value} onChange={(e) => onChange(e.target.value)} />
    </label>
  );
}
