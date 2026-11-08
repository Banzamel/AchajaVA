// The shared title bar (src/shared/ui/titlebar) with Achaja's icon and name.
import logo from '../assets/achaja.png';
import { windowFrame } from '../platform/index.ts';
import { useT } from '../shared/i18n/useT.ts';
import { TitleBar } from '../shared/ui/titlebar/index.ts';

export function AppTitleBar({ name }: { name: string }) {
  const t = useT();
  const labels = { minimize: t('window.minimize'), maximize: t('window.maximize'), restore: t('window.restore'), close: t('window.close') };
  return <TitleBar icon={<img src={logo} alt="" />} name={name} labels={labels} frame={windowFrame} />;
}
