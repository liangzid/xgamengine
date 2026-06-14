#!/usr/bin/env bash
# run.sh — Launch xgamengine CLI with proper NixOS library paths
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENGINE_DIR="$(dirname "$SCRIPT_DIR")"

# On NixOS, find the OpenSSL library path for cl+ssl
if [ -d /nix/store ]; then
    OPENSSL_LIB=$(find /nix/store -maxdepth 3 -name 'libcrypto.so' -path '*-openssl-*' 2>/dev/null | head -1 || true)
    if [ -z "$OPENSSL_LIB" ]; then
        OPENSSL_LIB=$(find /nix/store -maxdepth 3 -name 'libcrypto.so.3' -path '*-openssl-*' 2>/dev/null | head -1 || true)
    fi
    if [ -n "$OPENSSL_LIB" ]; then
        OPENSSL_LIB_DIR=$(dirname "$OPENSSL_LIB")
        export LD_LIBRARY_PATH="${OPENSSL_LIB_DIR}${LD_LIBRARY_PATH:+:}${LD_LIBRARY_PATH:-}"
    fi
fi

exec sbcl --noinform --script "$ENGINE_DIR/bin/cli.lisp" "$@"
