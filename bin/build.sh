#!/usr/bin/env bash
# build.sh — Build xgamengine Rust binary
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENGINE_DIR="$(dirname "$SCRIPT_DIR")"

# ---- Find OpenSSL on NixOS ----
if [ -d /nix/store ]; then
    OPENSSL_LIB=$(find /nix/store -maxdepth 3 -name 'libssl.so' -path '*-openssl-*' 2>/dev/null | head -1 || true)
    OPENSSL_DEV=$(find /nix/store -maxdepth 5 -name 'openssl.pc' -path '*-openssl-*' 2>/dev/null | head -1 || true)

    if [ -n "$OPENSSL_LIB" ]; then
        export OPENSSL_LIB_DIR="$(dirname "$OPENSSL_LIB")"
    fi
    if [ -n "$OPENSSL_DEV" ]; then
        OPENSSL_DEV_DIR="$(dirname "$(dirname "$(dirname "$OPENSSL_DEV")")")"
        export OPENSSL_INCLUDE_DIR="$OPENSSL_DEV_DIR/include"
        export PKG_CONFIG_PATH="$(dirname "$OPENSSL_DEV")"
    fi
fi

echo "==> Building xgamengine (release)..."
cd "$ENGINE_DIR"
cargo build --release

echo "==> Binary: $ENGINE_DIR/target/release/xgamengine"
ls -lh "$ENGINE_DIR/target/release/xgamengine"
