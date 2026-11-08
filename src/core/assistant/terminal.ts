// The Terminal tab: Claude's console lines, with the long separator lines (120 characters) cut so
// they do not wrap in a narrow window; and the keys offered for its choice dialogs.

export function terminalText(lines: string[]): string {
  return lines.map((l) => l.replace(/\s+$/, '').replace(/─{4,}/g, '────')).join('\n').replace(/^\n+/, '');
}

export const TERMINAL_KEYS: { label: string; keys: string[] }[] = [
  { label: '↑', keys: ['up'] },
  { label: '↓', keys: ['down'] },
  { label: 'Enter', keys: ['enter'] },
  { label: 'Esc', keys: ['esc'] },
  { label: '1', keys: ['text:1'] },
  { label: '2', keys: ['text:2'] },
  { label: '3', keys: ['text:3'] },
];

// A log line: its time and whether it reports an error (shown in red).
export function logLine(line: string): { time: string; text: string; error: boolean } {
  const m = /^(\d\d:\d\d:\d\d) (.*)$/s.exec(line);
  const text = m ? m[2] : line;
  return { time: m ? m[1] : '', text, error: /BLAD|BŁĄD|ERROR|Error|Traceback|Exception/.test(text) };
}
