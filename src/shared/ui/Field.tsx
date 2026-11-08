// A labelled text field (the label above it, like the settings always had).
import type { InputHTMLAttributes } from 'react';

interface Props extends Omit<InputHTMLAttributes<HTMLInputElement>, 'onChange'> {
  label: string;
  value: string;
  onChange: (value: string) => void;
}

export function Field({ label, value, onChange, className, ...rest }: Props) {
  return (
    <label className={`field ${className ?? ''}`}>
      <span className="field-label">{label}</span>
      <input className="input" value={value} onChange={(e) => onChange(e.target.value)} {...rest} />
    </label>
  );
}
