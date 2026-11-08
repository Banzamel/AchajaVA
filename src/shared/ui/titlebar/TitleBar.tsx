// The frameless window's title bar: app icon and name, the app's own status items, window buttons.
// Everything but the buttons drags the window; a double click maximizes or restores it.
import type { ReactNode } from 'react';
import type { TitleBarLabels, WindowFrame } from './types';
import { useWindowState } from './useWindowState';
import { WindowControls } from './WindowControls';
import './titlebar.css';

interface Props {
  icon: ReactNode;
  name: string;
  labels: TitleBarLabels;
  frame: WindowFrame;
  children?: ReactNode;
}

export function TitleBar({ icon, name, labels, frame, children }: Props) {
  const { maximized } = useWindowState(frame);
  return (
    <header className="titlebar" data-tauri-drag-region="deep">
      <div className="titlebar-brand">
        <span className="titlebar-icon">{icon}</span>
        <span className="titlebar-name">{name}</span>
      </div>
      <div className="titlebar-extra">{children}</div>
      <WindowControls frame={frame} labels={labels} maximized={maximized} />
    </header>
  );
}
