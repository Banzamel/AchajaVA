// `npm run dist`: builds the Windows installer in Docker (image tauri-build, see docker/build.sh)
// without Rust or MSVC on the host. Extra arguments go to `tauri build`.
import { spawnSync } from 'node:child_process';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const volumes = [
  `${root}:/app`,
  'achaja-node-modules:/app/node_modules',
  'achaja-target:/target',
  'achaja-cargo:/usr/local/cargo/registry',
  'achaja-xwin:/xwin',
];
const args = ['run', '--rm', ...volumes.flatMap((v) => ['-v', v]), '-w', '/app'];
for (const name of ['TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD']) {
  if (process.env[name] !== undefined) args.push('-e', name);
}
args.push('tauri-build', 'sh', 'docker/build.sh', ...process.argv.slice(2));
const result = spawnSync('docker', args, { stdio: 'inherit' });
process.exit(result.status ?? 1);
