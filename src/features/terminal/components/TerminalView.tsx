// The hidden Claude console, live, and keys for its choice dialogs (approvals, a new MCP server),
// shown only while such a dialog waits.
import { terminalText, TERMINAL_KEYS } from '../../../core/assistant/terminal.ts';
import type { Screen } from '../../../core/assistant/types.ts';
import { commands } from '../../../platform/index.ts';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { TextPanel } from '../../../shared/ui/TextPanel.tsx';

export function TerminalView({ screen, dialog }: { screen: Screen | null; dialog: boolean }) {
  const t = useT();
  const text = screen && screen.lines.length ? terminalText(screen.lines) : t('term.empty');
  return (
    <div className="tab-page">
      <div className="card grow-card"><TextPanel text={text} mono className="term" /></div>
      <div className="key-row">
        {dialog ? TERMINAL_KEYS.map((k) => (
          <Button key={k.label} variant="key" onClick={() => void commands.request(`keys:${k.keys.join(',')}`)}>{k.label}</Button>
        )) : null}
        <span className="grow" />
        <Button variant="key" onClick={() => void commands.toggleConsole()}>⧉  {t('term.console')}</Button>
      </div>
    </div>
  );
}
