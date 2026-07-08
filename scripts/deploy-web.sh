#!/usr/bin/env bash
# Deploy the web app to Vercel as a prebuilt static site.
#
# The wasm blob is built locally with `trunk build --release` (Vercel's build
# image has no Rust/trunk), then uploaded with `vercel deploy --prebuilt` so
# Vercel never recompiles Bevy.
#
# Usage:
#   scripts/deploy-web.sh            # deploy a preview
#   scripts/deploy-web.sh --prod     # deploy to production
#
# First run only: `vercel link` to create/connect the Vercel project.
set -euo pipefail

cd "$(dirname "$0")/.."

# `vercel build` reads vercel.json, runs the release trunk build, and writes the
# prebuilt output to .vercel/output.
vercel build "$@"
vercel deploy --prebuilt "$@"
