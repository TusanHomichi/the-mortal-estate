# Agent guide

**The Mortal Estate** is an original game about a family line, a land where
death stopped working, and a world that remembers. It's building its first
playable slice: [docs/DESIGN.md](docs/DESIGN.md) is the spec everything here
builds toward.

<!-- house:rules v1 begin. Copied from FieldmouseWorks/burrow house/rules.md, verbatim. Change it there, not here: burrow's house sync rewrites everything between these markers. -->
## House rules

These hold in every repo. This file's own sections add to them and win where
they say something more specific.

- Fix the whole cause. No workarounds, shims, fallbacks or half-migrations:
  when you replace something, delete the old path in the same change.
- One fact, one home. Each rule or fact lives in one place; link to it instead
  of restating it.
- Before you edit, say what done looks like and how it will be checked. Simple
  work gets a short plan, not a ceremony.
- Report only the checks you actually ran, and what they said. Skipped,
  unavailable or "should pass" is never a pass. Keep local and CI results
  apart.
- A new check comes with proof that it fails on the bug it's for.
- Touching a file over about 1,000 lines? Split it in that change.
- Rust by default. New code that's meant to last starts in Rust, unless its
  platform picks the language. New Python says why it isn't Rust, on a line in
  its PR body starting `Why not Rust:` (CI's rust-first check fails a PR that
  adds Python, or grows it past a fix, without one): a throwaway script, or a
  tool still being tuned, ported once it settles. Port existing Python
  whenever Rust buys anything, system and pipeline code first, and let the
  port redraw where the code lives and what owns it rather than copy it.
- The code is the memory. When something goes wrong, make the code, a type, a
  test or a check stop it happening again; a new rule in AGENTS.md is the last
  resort. Say in the PR which one you did.
- Keep going on work you were asked to do. Don't stop to ask again for what's
  already been asked for.
- Never commit what the repo keeps private: secrets, tokens, personal data, and
  anything listed under this file's "Private" section.
- `Closes #N` only when the issue is fully done; otherwise `Refs #N`.
- Tracked files never name a model. Which model does what is the machine's
  business, and model names go stale in weeks.

### Commits

Subjects are `area: plain sentence`, where the area is the module, directory or
tool the change is about: `autoland: merge only the head that was checked`.
Lowercase after the colon, no trailing period, no `feat:`/`fix:` prefixes. The
body, when there is one, says why.

Keep the git identity your machine gives you. Where commits are signed, the key
belongs to that identity: a signed commit under someone else's name and email
shows as Unverified on GitHub, and CI fails it.
<!-- house:rules end -->

## Working here

- **The design doc is the spec.** A change to the design edits
  [docs/DESIGN.md](docs/DESIGN.md) in the same pull request. Its
  [decisions table](docs/DESIGN.md#decisions) records the owner's calls; reversing
  or bending one waits for the owner. Everything else in the doc is a proposal
  that a playtest or a better argument can change.
- **Original only.** The repository is public. Nothing comes from another game:
  no names, text, maps, numbers or art. Describe the game on its own terms, never
  by comparison to another game. Third-party assets come only from openly
  licensed kits, with their licenses beside them.
- **Keep it small.** One design doc, this file and the README. No status
  ledgers, receipts or indexes of other docs: Git history and the pull requests
  are the record.

## Build and test

One Cargo workspace in `crates/`: `sim` is the rules and the world they run
(plain Rust, no engine in it), `content` compiles `content/` into what the
simulation runs on, `wire` is what the server and its clients say to each
other, `server` hosts a world, `link` is a client's side of the
connection, `bots` measures a hosted world under headless players, `client`
is the game a player runs (Bevy, pinned to one release) and `models` writes
the generated models into `assets/models/generated/` (a test fails when the
committed ones are stale; `cargo run -p tme-models` writes them). The client
links udev and Wayland: on a distro, their development packages and
pkg-config. `nix flake check` is the gate, the same locally and in CI:
rustfmt, clippy on all targets with warnings as errors, and the workspace's
tests, on the toolchain `rust-toolchain.toml` pins; and lychee, offline, over
every Markdown file, so every local link and `#fragment` must resolve. Plain
cargo runs the same Rust checks (`cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`).
`nix develop` has the toolchain and lychee.

## What needs the owner

- Changing a call in the [decisions table](docs/DESIGN.md#decisions).
- Publishing anything outside this repository (a build, a release, a hosted
  world, a public API, a content snapshot, a store page) or opening a world to
  people outside the owner's circle.
- Putting a hidden-world answer, the truth behind one of the world's
  mysteries, in this repository or anywhere else public.
- Spending money.

Everything else follows the house rules above.

<!-- house:prs v1 begin. Copied from FieldmouseWorks/burrow house/prs.md, verbatim. Change it there, not here: burrow's house sync rewrites everything between these markers. -->
## Pull requests

Open PRs ready for review, never as drafts, and leave them to the steward: it
asks the review bot for a review, and once the bot finds nothing on your head,
every review thread is resolved and CI is green, it merges the PR. Don't merge
it yourself. Fix or answer every finding: push the fix and reply with the
commit, or reply with why it's wrong, then resolve the thread. A PR touching a
one-way path (secrets, CODEOWNERS) gets a judgment first: it lands unless it
can't be undone or is the owner's to decide.
<!-- house:prs end -->

## Code Review Rules

For reviewers, the review bot included. Flag as P1:

- a change that makes a check pass without fixing the thing it checks
- tests or checks weakened, skipped or deleted to get green
- secrets, tokens or personal data in the diff or the logs it adds
- behavior changed beyond what the PR says it does
- a call in the design doc's decisions table changed or contradicted without
  the owner's recorded say-so
- another game's names, text, maps, numbers or art, or the game described by
  comparison to another game
- a third-party asset without its license beside it
- a hidden-world answer (the truth behind one of the world's mysteries) in the
  diff without the owner's recorded say-so

## Private

Never committed, never quoted in issues, PRs or logs:

- Credentials of any kind.
