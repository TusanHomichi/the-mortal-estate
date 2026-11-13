# Agent guide

**The Mortal Estate** is an original game about a family line, a land where
death stopped working, and a world that remembers. It's building its first
playable slice: [docs/DESIGN.md](docs/DESIGN.md) is the spec everything here
builds toward.

<!-- house:rules v1 begin. Copied from FieldmouseWorks/burrow house/rules.md, verbatim. Change it there, not here: burrow's house sync rewrites everything between these markers. -->
## House rules

These hold in every repo. This file's own sections add to them and win where
they're more specific.

- Design the class, not the instance. When a bug, finding or stall shares a
  cause with others, fix the cause once, where it lives (one owning path, a
  type, a check), so none of its cases come back.
- Do work once. Look for what's already open or built first, extend the
  shared crate, client or job that does it rather than write another copy,
  and never run work twice that once would do.
- Fix the whole cause: no workarounds, shims, fallbacks or half-migrations.
  When you replace something, delete the old path in the same change.
- One fact, one home. Each rule or fact lives in one place; link to it
  instead of restating it.
- Pipeline options live in the settings source: change them through its
  door, read them with `proj settings show`, and link the
  [settings reference](https://github.com/FieldmouseWorks/burrow/blob/main/docs/settings-reference.md)
  rather than restate a value or where one is kept.
- Before you edit, say what done looks like and how it will be checked.
- Report only the checks you ran and what they said. Skipped, unavailable
  or "should pass" is never a pass. Keep local and CI results apart.
- A new check comes with proof that it fails on the bug it's for.
- The code is the memory. When something goes wrong, make the code, a type,
  a test or a check stop it happening again; a new rule here is the last
  resort. Say in the PR which one you did.
- Keep going on work you were asked to do: don't ask again for what's
  already been asked for.
- Never commit what the repo keeps private: secrets, tokens, personal data,
  and anything under this file's "Private" section.
- `Closes #N` only when the issue is fully done; otherwise `Refs #N`.

CI's house check holds every PR to its title's style and to the rules below,
each of which a PR can pass with a line in its body starting as shown and a
reason of four words or more. Editing the body or title runs it again.

- Rust by default, shell included: new code meant to last starts in Rust
  unless its platform picks the language, and a coin toss goes to Rust. A PR
  that adds Python, or grows Python and shell past a fix, says `Why not
  Rust:` (a file a tool generates, glue that only runs another program, a
  throwaway script, a tool still being tuned). Port existing Python and
  shell whenever Rust buys anything, system and pipeline code first.
- Work starts from an event (a push, a webhook, a unit finishing), not a
  clock. A PR that adds a timer or a cron schedule says `Why a timer:`.
- Split a code file before it passes 1,000 lines, or say `Why not split:`.
- AGENTS.md is a map: what the repo is, its own rules, and where things live
  (a feature map, the doc that owns each subject). Detail goes in the doc
  that owns it, linked from here. A PR that grows the repo's own part (the
  lines outside these blocks) past 100 lines says `Why a longer guide:`.
- Tracked files never name a model: which model does what is the machine's
  business, and names go stale in weeks. A line that has to (a recorded
  screen in a test) says `Why a model name:`.

### Commits

Subjects are `area: plain sentence`, the area being the module, directory or
tool the change is about (`autoland: merge only the head that was checked`):
lowercase after the colon, no trailing period, no `feat:`/`fix:`. The body,
if any, says why. A PR's title is the subject main gets, so it's written the
same way. Keep the git identity your machine gives you: a commit signed with
a key that isn't its author's shows as Unverified, and CI fails it.
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

One task, one PR: finish what the task set out to do, then open one PR, so
it goes through review and CI once. No PR per step or round, and no stacks.
A fix to the pipeline (the steward, CI, the machine) lands as soon as it's
ready, and so does a change to these rules that the work needs, in a PR of
its own: reviewers read the default branch's rules, not the PR's.

Open PRs ready for review, never as drafts, and leave them to the steward:
once the review bot finds nothing on your head, every thread is resolved and
CI is green, it merges. Don't merge it yourself. Answer every finding: push
the fix and reply with the commit, or reply with why it's wrong, then
resolve the thread. A PR touching a one-way path (secrets, CODEOWNERS) gets
a judgment first, and lands unless it can't be undone or is the owner's
call.

A red default branch is the steward's to fix, from a branch named
`autoland/red-` and the commit. Don't fix it yourself, in any repo, unless
the pipeline's stuck issue says the steward's jobs gave up; then fix it from
a branch of that name.
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
