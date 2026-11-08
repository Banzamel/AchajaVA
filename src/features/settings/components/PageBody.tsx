// The page of the current step / section.
import type { ReactNode } from 'react';
import type { PageKey } from '../../../core/settings/types.ts';
import { AgentsPage } from '../pages/AgentsPage.tsx';
import { GeneralPage } from '../pages/GeneralPage.tsx';
import { HaPage } from '../pages/HaPage.tsx';
import { MailPage } from '../pages/MailPage.tsx';
import { RulesPage } from '../pages/RulesPage.tsx';
import { SummaryPage } from '../pages/SummaryPage.tsx';
import { VoicePage } from '../pages/VoicePage.tsx';

const PAGES: Record<PageKey, () => ReactNode> = {
  general: GeneralPage,
  voice: VoicePage,
  agents: AgentsPage,
  mail: MailPage,
  rules: RulesPage,
  ha: HaPage,
  summary: SummaryPage,
};

export function PageBody({ page }: { page: PageKey }) {
  const Page = PAGES[page];
  return (
    <div className="settings-body">
      <Page />
    </div>
  );
}
