#!/usr/bin/env bash
# State the runner's disk budget, then widen it — before anything spends it.
#
# On 2026-08-20 run 32438837232 died twice with
# `System.IO.IOException: No space left on device`, twelve minutes in, with no
# step log written at all. The failure said nothing about how much room the job
# had started with or where it had gone, because nothing in the job had ever
# said either.
#
# So this runs first in every job that builds, and it prints `df` on both sides
# of the reclamation. The four directories below are GitHub-hosted image
# payloads this repository has no use for — a .NET SDK, an Android SDK, a
# Haskell toolchain, and a CodeQL bundle. Removing them is not a workaround for
# a lane that is too big: the lane is measured to fit without it (see
# docs/agent-workflow.md, "The disk budget"), and this is the margin that keeps
# a fit from becoming a coin flip when the image grows.
#
# It never fails the job. A runner image that has already dropped one of these
# is a smaller reclamation, not a broken build — which is why the removals are
# tolerant and the budget is printed rather than asserted. What must not happen
# is a job spending disk it never counted.
set -u

echo "--- disk before"
df -h /

for payload in \
  /usr/share/dotnet \
  /usr/local/lib/android \
  /opt/ghc \
  /usr/local/.ghcup \
  /opt/hostedtoolcache/CodeQL
do
  if [ -e "$payload" ]; then
    echo "removing $payload"
    sudo rm -rf "$payload" || echo "could not remove $payload; continuing"
  else
    echo "absent already: $payload"
  fi
done

echo "--- disk after"
df -h /
