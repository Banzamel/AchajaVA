// Entry of Achaja's one window (index.html); settings are a view inside it.
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ServicesProvider } from '../shared/hooks/services.tsx';
import '../shared/ui/titlebar/titlebar.css';
import '../styles/index.css';
import { MainApp } from './MainApp.tsx';
import { createServices } from './services.ts';
import { showWhenDrawn, startApp } from './startApp.ts';

const registry = createServices();
const root = document.getElementById('root');

void startApp(registry)
  .catch((e: unknown) => console.error(e))
  .finally(() => {
    if (!root) return;
    createRoot(root).render(
      <StrictMode>
        <ServicesProvider registry={registry}>
          <MainApp />
        </ServicesProvider>
      </StrictMode>,
    );
    showWhenDrawn(new URLSearchParams(window.location.search).has('minimized'));
  });
