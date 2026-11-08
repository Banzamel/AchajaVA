// Versioning helper. package.json "version" is the single source of truth.
//   node scripts/version.js stamp        — (npm "version" hook) moves "Unreleased" changelog
//                                          entries into a "## [x.y.z] - date" section
//   node scripts/version.js check [tag]  — verifies the tag (or GITHUB_REF_NAME) matches
//                                          package.json and that CHANGELOG.md has the version
const fs = require('fs');
const path = require('path');

const root = path.join(__dirname, '..');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const changelogPath = path.join(root, 'CHANGELOG.md');
const fail = (msg) => { console.error(`✖ ${msg}`); process.exit(1); };

function stamp() {
  const log = fs.readFileSync(changelogPath, 'utf8');
  if (log.includes(`## [${pkg.version}]`)) return console.log(`CHANGELOG already has ${pkg.version}`);
  const m = log.match(/## \[Unreleased\]\s*\n([\s\S]*?)(?=\n## \[|$)/);
  if (!m || !m[1].trim()) fail('CHANGELOG.md: add your changes under "## [Unreleased]" before bumping the version.');
  const date = new Date().toLocaleDateString('sv-SE'); // local YYYY-MM-DD
  let next = log.replace(m[0], `## [Unreleased]\n\n## [${pkg.version}] - ${date}\n\n${m[1].trim()}\n`);
  // Keep the comparison links at the bottom in sync.
  const repo = 'https://github.com/Banzamel/AchajaVA';
  next = next.replace(/^\[Unreleased\]: .*$/m, `[Unreleased]: ${repo}/compare/v${pkg.version}...HEAD`);
  const prev = (log.match(/^## \[(\d+\.\d+\.\d+[^\]]*)\]/m) || [])[1];
  const link = prev ? `${repo}/compare/v${prev}...v${pkg.version}` : `${repo}/releases/tag/v${pkg.version}`;
  next = next.replace(/^(\[Unreleased\]: .*)$/m, `$1\n[${pkg.version}]: ${link}`);
  fs.writeFileSync(changelogPath, next);
  console.log(`CHANGELOG: released ${pkg.version} (${date})`);
}

function check(tagArg) {
  if (!/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(pkg.version)) fail(`package.json version "${pkg.version}" is not SemVer`);
  const tag = tagArg || process.env.GITHUB_REF_NAME;
  if (tag && tag !== `v${pkg.version}`) fail(`tag ${tag} does not match package.json version v${pkg.version}`);
  if (!fs.readFileSync(changelogPath, 'utf8').includes(`## [${pkg.version}]`)) fail(`CHANGELOG.md has no "## [${pkg.version}]" section`);
  console.log(`✔ version ${pkg.version}${tag ? ` matches tag ${tag}` : ''}`);
}

// Writes the changelog section of the current version to a file (GitHub release body).
function notes(out) {
  const log = fs.readFileSync(changelogPath, 'utf8');
  const m = log.match(new RegExp(`## \\[${pkg.version.replace(/\./g, '\\.')}\\][^\\n]*\\n([\\s\\S]*?)(?=\\n## \\[|\\n\\[[^\\]]+\\]: |$)`));
  if (!m) fail(`CHANGELOG.md has no "## [${pkg.version}]" section`);
  const body = `${m[1].trim()}\n\n**Download:** \`AchajaSetup-${pkg.version}.exe\` below. Installed copies update themselves (also the Python versions up to 0.13.2).\n`;
  fs.mkdirSync(path.dirname(path.resolve(out)), { recursive: true });
  fs.writeFileSync(out, body);
  console.log(`release notes for ${pkg.version} -> ${out}`);
}

const [cmd, arg] = process.argv.slice(2);
if (cmd === 'stamp') stamp();
else if (cmd === 'check') check(arg);
else if (cmd === 'notes') notes(arg || 'release-notes.md');
else fail('usage: node scripts/version.js stamp | check [tag] | notes [file]');
