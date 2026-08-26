"""Workbench V0 — the Selection Bridge.

The owner points, at the compiled logical surface or at a real gameplay capture,
and an agent receives an exact, stable, machine-resolvable address. Both surfaces
answer in one address space. That is the whole of V0.

This package is a **tool**. It is never a runtime input, never a second
gameplay authority, never a second content ledger, and never a second
renderer: the logical view draws the authoring compiler's own emitted
projection and nothing else, and it says so on screen.

Nothing here mutates anything. There are no staged operations, no Apply, no
candidate validation, no image operations, and no promotion path. The session
directory is disposable working state under an ignored root.

One thing here starts a program: taking a new capture runs the shipped client,
because a picture of what the client draws can only come from the client drawing
it. It lives alone in `capture_harness`, off the selection path, and it is the
only exception.
"""

from __future__ import annotations

VERSION = "v0"
