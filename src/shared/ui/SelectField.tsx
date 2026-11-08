// A labelled choice from a list only (no own text), as the settings' comboboxes were.
interface Option {
  value: string;
  label: string;
}

interface Props {
  label: string;
  value: string;
  options: Option[];
  onChange: (value: string) => void;
  className?: string;
}

export function SelectField({ label, value, options, onChange, className }: Props) {
  return (
    <label className={`field ${className ?? ''}`}>
      <span className="field-label">{label}</span>
      <select className="input select" value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
    </label>
  );
}
