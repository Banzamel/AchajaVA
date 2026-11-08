// The ☰ menu (also F10): large, readable entries with icons in the theme's colours.
import { useEffect, useRef, useState } from 'react';
import { useService } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { useMenuActions } from '../hooks/useMenuActions.ts';
import { AssistantService } from '../AssistantService.ts';

export function AppMenu() {
  const t = useT();
  const svc = useService(AssistantService);
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const items = useMenuActions(svc);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'F10') {
        e.preventDefault();
        setOpen((o) => !o);
      }
      if (e.key === 'Escape') setOpen(false);
    };
    const onClick = (e: MouseEvent) => {
      if (box.current && !box.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('mousedown', onClick);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('mousedown', onClick);
    };
  }, []);
  return (
    <div className="menu-anchor" ref={box}>
      <Button variant="ghost" title={t('menu.tip')} onClick={() => setOpen((o) => !o)}>☰</Button>
      {open ? (
        <div className="popup-menu card">
          {items.map((item, i) =>
            item ? (
              <button key={item.label} type="button" className="menu-item" onClick={() => { setOpen(false); item.run(); }}>
                <span className="menu-icon">{item.icon}</span>
                <span>{item.label}</span>
              </button>
            ) : (
              <div key={`sep${i}`} className="menu-sep" />
            ),
          )}
        </div>
      ) : null}
    </div>
  );
}
