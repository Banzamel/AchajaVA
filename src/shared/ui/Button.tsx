// A button in Achaja's look: normal, accent (dark inside, neon frame), ghost (no background),
// key (the small keys of the Terminal tab and the lists).
import type { ButtonHTMLAttributes } from 'react';

type Variant = 'normal' | 'accent' | 'ghost' | 'key' | 'chip';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  selected?: boolean;
}

export function Button({ variant = 'normal', selected, className, type, ...rest }: Props) {
  const cls = ['btn', `btn-${variant}`, selected ? 'selected' : '', className ?? ''].filter(Boolean).join(' ');
  return <button type={type ?? 'button'} className={cls} {...rest} />;
}
