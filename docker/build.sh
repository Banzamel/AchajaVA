#!/bin/sh
# Builds the Windows installer inside the tauri-build image (Linux + cargo-xwin + NSIS), so the
# host needs neither Rust nor MSVC. Run through `npm run dist` (scripts/docker-build.mjs), which
# mounts the repository at /app with named volumes for node_modules, the cargo registry, the
# MSVC SDK cache and the build target. The setup ends up in release/.
set -eu
cd /app

# The Tauri CLI on a Linux host looks for an appindicator library whenever the tray feature is on,
# even for a Windows target (through pkg-config); a placeholder satisfies the check (never loaded).
PC=/usr/lib/x86_64-linux-gnu/pkgconfig
mkdir -p "$PC"
printf 'libdir=/usr/lib/x86_64-linux-gnu\nName: ayatana-appindicator3-0.1\nDescription: placeholder\nVersion: 0.5.92\nLibs: -L${libdir}\n' > "$PC/ayatana-appindicator3-0.1.pc"
touch /usr/lib/x86_64-linux-gnu/libayatana-appindicator3.so.1

npm ci --no-audit --no-fund
npm run fetch-vosk
npm test
(cd src-tauri && cargo test -p achaja-core)
npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis "$@"

version=$(node -p "require('./package.json').version")
bundle=/target/x86_64-pc-windows-msvc/release/bundle/nsis
mkdir -p release
cp "$bundle/Achaja_${version}_x64-setup.exe" "release/AchajaSetup-${version}.exe"
if [ -f "$bundle/Achaja_${version}_x64-setup.exe.sig" ]; then
  npm run release:manifests -- "$bundle"
fi
cp /target/x86_64-pc-windows-msvc/release/Achaja.exe release/Achaja.exe
ls -l release
