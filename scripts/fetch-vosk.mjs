// `npm run fetch-vosk`: the Vosk library for Windows (libvosk.dll and its MinGW runtime DLLs)
// into resources/vosk/, which the installer ships. Downloaded once; the folder is git-ignored.
import fs from 'node:fs';
import path from 'node:path';
import { unzipSync } from 'fflate';

const VERSION = '0.3.45';
const URL = `https://github.com/alphacep/vosk-api/releases/download/v${VERSION}/vosk-win64-${VERSION}.zip`;
const root = path.resolve(import.meta.dirname, '..');
const out = path.join(root, 'resources', 'vosk');
const marker = path.join(out, 'libvosk.dll');

if (fs.existsSync(marker)) {
  console.log(`resources/vosk: libvosk ${VERSION} already there`);
  process.exit(0);
}
const res = await fetch(URL);
if (!res.ok) {
  console.error(`✖ ${URL}: HTTP ${res.status}`);
  process.exit(1);
}
const files = unzipSync(new Uint8Array(await res.arrayBuffer()));
fs.mkdirSync(out, { recursive: true });
let n = 0;
for (const [name, data] of Object.entries(files)) {
  if (!name.toLowerCase().endsWith('.dll')) continue;
  fs.writeFileSync(path.join(out, path.basename(name)), data);
  n += 1;
}
if (!fs.existsSync(marker)) {
  console.error('✖ libvosk.dll not found in the archive');
  process.exit(1);
}
console.log(`resources/vosk: ${n} DLLs of Vosk ${VERSION}`);
