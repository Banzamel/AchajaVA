// `npm run release:manifests [bundle dir]`: puts the signed NSIS setup into release/ under its
// release name (AchajaSetup-<version>.exe — the name the Python versions' updater looks for, so
// they update straight into this installer) and writes latest.json for the Tauri updater, with
// the setup's minisign signature inside it. Only these two files are published.
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const version = pkg.version;
const bundle = process.argv[2] || path.join(root, 'src-tauri', 'target', 'release', 'bundle', 'nsis');
const setupName = `AchajaSetup-${version}.exe`;
const out = path.join(root, 'release');
const fail = (msg) => { console.error(`✖ ${msg}`); process.exit(1); };

const built = path.join(bundle, `Achaja_${version}_x64-setup.exe`);
if (!fs.existsSync(built)) fail(`no installer at ${built}`);
if (!fs.existsSync(`${built}.sig`)) fail(`no signature at ${built}.sig (build with createUpdaterArtifacts and TAURI_SIGNING_PRIVATE_KEY)`);

fs.mkdirSync(out, { recursive: true });
fs.copyFileSync(built, path.join(out, setupName));
const notesFile = path.join(root, 'release-notes.md');
const latest = {
  version,
  notes: fs.existsSync(notesFile) ? fs.readFileSync(notesFile, 'utf8') : '',
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: fs.readFileSync(`${built}.sig`, 'utf8').trim(),
      url: `https://github.com/Banzamel/AchajaVA/releases/download/v${version}/${setupName}`,
    },
  },
};
fs.writeFileSync(path.join(out, 'latest.json'), `${JSON.stringify(latest, null, 2)}\n`);
console.log(`release/ ready: ${setupName} (${fs.statSync(path.join(out, setupName)).size} bytes), latest.json`);
