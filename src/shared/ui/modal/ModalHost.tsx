// Draws the current question of the ModalService; Escape answers "no".
import { useEffect } from 'react';
import { useService, useStore } from '../../hooks/services.tsx';
import { Button } from '../Button.tsx';
import { ModalService } from './ModalService.ts';

export function ModalHost() {
  const modal = useService(ModalService);
  const s = useStore(modal.store);
  useEffect(() => {
    if (!s.open) return undefined;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') modal.answer(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [s.open, modal]);
  if (!s.open) return null;
  return (
    <div className="modal-backdrop">
      <div className="modal card" role="dialog">
        <p className="modal-text">{s.text}</p>
        <div className="modal-buttons">
          {s.buttons.map((b) => (
            <Button key={b.label} variant={b.accent ? 'accent' : 'normal'} onClick={() => modal.answer(b.value)} autoFocus={b.accent}>
              {b.label}
            </Button>
          ))}
        </div>
      </div>
    </div>
  );
}
