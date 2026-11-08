// Project folders: every subfolder of these is a project Achaja can hand work to.
import { useState } from 'react';
import { pickFolder } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { SettingsService } from '../SettingsService.ts';

export function AgentsPage() {
  const t = useT();
  const svc = useService(SettingsService);
  const { form } = useStore(svc.store);
  const [selected, setSelected] = useState<string[]>([]);
  if (!form) return null;
  const add = async () => {
    const path = await pickFolder();
    if (path && !form.project_roots.includes(path)) svc.update((f) => ({ ...f, project_roots: [...f.project_roots, path] }));
  };
  const remove = () => {
    svc.update((f) => ({ ...f, project_roots: f.project_roots.filter((r) => !selected.includes(r)) }));
    setSelected([]);
  };
  const toggle = (root: string, add: boolean) =>
    setSelected((s) => (add ? (s.includes(root) ? s.filter((x) => x !== root) : [...s, root]) : [root]));
  return (
    <div className="side-layout">
      <div className="card list-card tall">
        {form.project_roots.map((r) => (
          <button type="button" key={r} className={`list-row ${selected.includes(r) ? 'selected' : ''}`} onClick={(e) => toggle(r, e.ctrlKey || e.shiftKey)}>{r}</button>
        ))}
      </div>
      <div className="side-buttons">
        <Button variant="accent" onClick={() => void add()}>{t('set.add_folder')}</Button>
        <Button disabled={!selected.length} onClick={remove}>{t('set.remove')}</Button>
      </div>
    </div>
  );
}
