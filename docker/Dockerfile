# Build environment for the Windows installer, without Rust or MSVC on the host:
# Linux + cargo-xwin (MSVC CRT/SDK downloaded by xwin) + NSIS, as in Tauri's cross-compilation guide.
FROM rust:1.90-bookworm

RUN apt-get update \
 && apt-get install -y --no-install-recommends nsis lld llvm clang curl ca-certificates \
 && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

# Debian ships NSIS 3.08, whose headers lack Win/RestartManager.nsh (used by the Tauri template).
RUN curl -fsSL https://raw.githubusercontent.com/kichik/nsis/v311/Include/Win/RestartManager.nsh -o /usr/share/nsis/Include/Win/RestartManager.nsh

RUN rustup target add x86_64-pc-windows-msvc \
 && cargo install --locked cargo-xwin

ENV CARGO_TARGET_DIR=/target \
    XWIN_CACHE_DIR=/xwin
WORKDIR /app
