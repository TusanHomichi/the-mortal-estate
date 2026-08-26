---
last_updated: 2026-08-19
revision: 1
status: Built at Phase 3, before any payload; pending owner acceptance at G3.
public_safe: true
summary: The public-boundary checks — what each defends, its fail-closed semantics, and the P9 mutant kills that qualify it to block.
routes:
  - tools/check_*.py
  - tools/boundary_common.py
  - tools/run_checks.py
  - tools/*-allowlist.txt
  - tools/ci-synthetic-banned-terms.txt
  - tests/test_check_*.py
  - tests/test_boundary_common.py
  - tests/test_run_checks.py
---

# Public-Boundary Checks

## Status

These are The Mortal Estate's public-boundary checks. They were built in
Phase 3, **before any content, rules, client, or tooling payload arrived**, and
that ordering is the point. A boundary check written after the payload lands is
written by someone who already knows what the payload contains, and it gets
shaped to pass it. These were written against an empty tree, so every later port
is checked on arrival instead of audited afterward.

They are qualified under `authoring-contracts.md` **P9**: a check earns blocking
status only by killing a deliberate mutant, and runs advisory until it does. The
table in [Qualification](#qualification) records every mutant and the exact test
that kills it. Pending owner acceptance at G3.

## The checks

| Check | Tool | Defends against |
| --- | --- | --- |
| banned-terms | `tools/check_boundary_terms.py` | source-lineage proper nouns and retired predecessor identities reaching a public surface, in file contents or file names |
| review-refs | `tools/check_review_refs.py` | content provenance chains that point at records the tree does not carry |
| hostnames | `tools/check_hostnames.py` | real external infrastructure named in carried tests, fixtures, and configuration |
| clean-room | `tools/check_clean_room.py` | dependence on a predecessor-private root, and private roots that could be committed by accident |
| markdown-links | `tools/check_markdown_links.py` | a carried document promising an authority and delivering nothing — a relative link or heading anchor that resolves to no carried file |

Run them together:

```bash
python3 tools/run_checks.py
```

`tools/run_checks.py` **owns the registry** — the single list of which checks
exist and where each lives. `tools/run_verification.py` reads that list to build
one runner step per check rather than restating it, so a check joins the
verification runner in the same edit that registers it and cannot be half-added.
The `boundary` lane runs the first four; the `docs` lane owns `markdown-links`,
because a documentation-only change must run it and must not pay for anything
else.

There is one check outside this family, because it is about the runner rather
than the boundary: **step-targets**
(`python3 tools/run_verification.py --check-step-targets`) asserts that every
script, module, binary, client resource, capability, and owner scope the step
table names actually exists. It is qualified the same way and its receipts are
in the table below.

Run the tests:

```bash
python3 -m unittest discover -s tests
```

## What a check scans

Every check scans the same file set, defined once in `tools/boundary_common.py`:
the files git carries **or would carry** — tracked files plus untracked files
that are not ignored. Scanning only `git ls-files` would leave a violation
unexamined in the working tree until the moment it was committed, which is the
one moment the check stops being useful. Ignored files are excluded because they
are, by construction, the private side of the boundary.

File contents are read as UTF-8 text. Binary files are detected by a null-byte
sniff and their contents are skipped — but their **names** are still checked,
because a filename carries meaning whatever the bytes say.

## Fail-closed semantics

Exit codes are shared by every check, so a caller can tell a dirty tree from a
broken check:

| Code | Meaning |
| --- | --- |
| 0 | clean |
| 1 | violations found — the tree is wrong |
| 2 | usage error |
| 3 | **FAIL CLOSED** — the check could not run as specified |

Exit 3 is never a skip and never a pass. A check whose configuration input is
missing, unreadable, not valid UTF-8, or empty after comments are stripped
reports FAIL CLOSED and stops. An empty data file is a broken input, not an
empty policy. A boundary check that goes quiet when its input disappears is
worse than no check, because it reports green while defending nothing — the
false-green class `authoring-contracts.md` §1 exists to kill.

In `tools/run_checks.py`, a fail-closed result outranks a violation in the
summary: if any check could not run, you cannot trust the rest of the run.

## The private-terms convention

The predecessor shipped its denylist as a literal array inside public code,
which named the very lineage the denylist existed to keep out of public
surfaces. This project inverts that:

- **The mechanism is public.** `tools/check_boundary_terms.py` and its tests are
  carried, reviewable, and complete.
- **The terms are private.** They live in `.boundary/banned-terms.txt`, a
  git-ignored root. The file is never carried and never enters history.
- **A missing term file fails the check closed** (exit 3). This is what makes
  the convention safe: the private half cannot be lost quietly. A fresh clone
  has no `.boundary/` root and every run of the banned-term check exits 3 until
  the owner provisions the file out of band. That is the intended first
  experience, not a setup bug.
- **A synthetic fixture proves the mechanism.**
  `tests/fixtures/synthetic-terms.txt` carries invented nonsense terms. Every
  test — loading, matching, word boundaries, separator tolerance, filename
  scanning, fail-closed behavior — runs against those, so the carried tree
  proves the rejection behavior without carrying a single real term.

The term-matching rule is case-insensitive with word-ish boundaries: a match
must not be flanked by an alphanumeric character, so a short term never fires
from inside a longer word. Within a term, any run of whitespace or punctuation
matches any run of non-alphanumeric characters including none, so a two-word
term also catches its dotted, underscored, hyphenated, and camel-cased forms.
The rule deliberately errs toward catching. A false positive costs ten seconds
of human review; a false negative ships.

The same discipline binds the code in this repository: **no real banned term
appears in any carried file**, including this document, the tools, and the
tests. During construction the banned-terms check caught its own tool's
docstring using real terms as examples, which is the mechanism working exactly
as intended.

### The second enforcement point: the content validator

`tools/check_boundary_terms.py` scans what the repository **carries**. It cannot
see content that never enters the tree — a scenario passed to the simulator on
the command line, or content a future authoring tool emits. So the rules crate
carries the same convention at **load time**, in
`crates/tme-rules/src/content/validation/boundary/terms.rs`:

- Same file format, same matching rule, same fail-closed semantics. A missing,
  unreadable, or entry-less term file makes every clean-content validation fail
  with a diagnostic naming the cause.
- The file is named by the `TME_BANNED_TERMS_FILE` environment variable, or
  found as `.boundary/banned-terms.txt` by walking up from the working
  directory.
- `.cargo/config.toml` points cargo-run processes at the tracked synthetic
  fixture with `force = false`, so a value already in the environment wins. That
  is what lets a clean clone build and test green with no `.boundary/` root, and
  it is why the runtime and CI path exports the real file explicitly.

One consequence is worth stating plainly, because it looks like a bug and is
not. Three negative fixtures in the Rust suite assert that specific **synthetic**
terms are rejected. Run the suite with `TME_BANNED_TERMS_FILE` pointing at the
real private list and those three fail, because the real list does not contain
nonsense words. That is the convention working: a tree carrying no real term
cannot write a fixture the real list rejects. Every other test — including all
of the content corpus validation, which is the proof that the tree carries no
real term — passes under either list.

## Allowlists

Three of the checks take a tracked allowlist. Every entry carries a `#` comment
giving the reason it exists, and the lists hold what the tree actually contains
rather than what it might contain later.

- `tools/hostname-allowlist.txt` — hosts and IPv4 literals accepted anywhere in
  the tree. Today: the two hosts in the AGPL license text, the two named by the
  Cargo dependency metadata and the comment explaining it, and the seven
  synthetic strings that exist only as this check's own mutants inside
  `tests/test_check_hostnames.py`. Reserved names (`.invalid`, `.test`,
  `.example`, `.localhost`, the `example` domains), loopback addresses, and the
  RFC 5737 documentation ranges are allowed by rule and need no entry.
- `tools/clean-room-allowlist.txt` — the files permitted to name a
  predecessor-private root. Only files that define, ignore, document, or prove
  the rule. A **stale entry naming a file the tree does not carry is a
  violation**, because a file exemption that nobody is watching is how a scan
  gap opens.

The hostname allowlist has no stale-entry rule and the clean-room allowlist
does. That asymmetry is deliberate: a clean-room entry exempts a whole file from
scanning, so it must stay earned, while a hostname entry exempts one literal
string and cannot widen.

The mutant hostnames are listed in the allowlist rather than hidden by splicing
string literals in the test source. A check you can dodge by concatenating
constants is not a check, and the kills themselves are proven against an empty
allowlist regardless.

## Hostname tier 3 and identifier vocabulary

The hostname check reads bare dotted names (tier 3) by their final label,
because a bare dotted name is otherwise indistinguishable from a filename. When
the rules, protocol, and authoring crates arrived, that produced **896
violations, every one a false positive**: `engine.world` (799), `item.name`
(46), `origin.site` (36), `edge.at` (8), `self.live` (5), all field access in
Rust, plus two genuine prose mentions of the Cargo registry host, which were
allowlisted.

The fix extends a principle the check already had. Labels that double as this
project's **file extensions** were always absent from the tier-3 set; labels
that double as ordinary **code-identifier vocabulary** now are too, listed in
`EXCLUDED_IDENTIFIER_LABELS` with their measured counts. The exclusion is
applied by subtraction from the full TLD vocabulary rather than by deleting
entries, so re-adding a label cannot silently undo it, and two tests assert the
sets stay disjoint and that every excluded label was really in the vocabulary.

Giving up a label costs only tier 3. A real host under `.world` or `.name` still
dies in tier 1 (`scheme://host`, `user@host`) or tier 2 (`//host`, `host:port`),
and that is not an assertion — it is
`test_mutant_excluded_label_still_dies_in_url_form`. Tier 3 keeps its own
standing mutant so a future trim cannot hollow it out unnoticed.

**Known limitation, recorded rather than fixed.** Label matching is a proxy for
the real question: is this dotted name a string a human wrote, or an expression
the compiler reads? The sharper rule is to fire tier 3 only inside quoted
strings and in non-source files. It would end this class of false positive
outright instead of one label at a time, and it would cost coverage of bare
hosts in code comments. It changes detection semantics, so it needs its own
mutants and an owner decision; until then, expect the occasional new label to
need excluding, with the measurement in this section as the precedent for how.
**Two labels were excluded on that precedent when Workbench V1 landed**:
`contracts.no_executor` (3, a module attribute) and `arguments.click` (2, an
argparse attribute in a tool whose whole subject is pointing at things). Both are
ordinary code-identifier vocabulary here; neither is a host anybody wrote.

Every exclusion is evidence-driven: measure the false positives, exclude the
labels that produced them, and prove tiers 1 and 2 still cover the gap.

## Provenance resolution

`review_refs` entries are resolved, not merely counted. An entry must be a
non-empty repository-relative string; a trailing `#fragment` is stripped; the
result must be a carried file, or a directory containing at least one carried
file. Absolute paths and paths escaping the root are violations. A carried
`.json` file that does not parse is a violation, because a file whose provenance
cannot be read cannot be proven clean.

With no content in the tree the check is trivially green, and that is the
correct result. It exists so that the first orphaned reference fails **on
arrival** in Phase 4, rather than surfacing six phases later as a discovery.

## Qualification

Every check below has killed at least one deliberate mutant, so every check
blocks. Mutants live only in temporary git repositories built by the tests;
none is ever written into this tree.

**Standing rule: a new boundary check runs ADVISORY until its mutant kill is
recorded in this table.** Adding a check without adding its row means adding a
check that does not block yet. Recording a kill means naming the exact test, so
the claim stays checkable by running it.

### banned-terms

| Mutant | Killed by |
| --- | --- |
| a banned term in a carried file's contents | `test_check_boundary_terms.Mutants.test_mutant_term_in_file_contents_is_killed` |
| a banned term in a carried file's path | `test_check_boundary_terms.Mutants.test_mutant_term_in_file_path_is_killed` |
| a banned term in a file not yet tracked but committable | `test_check_boundary_terms.Mutants.test_mutant_term_in_untracked_but_committable_file_is_killed` |
| a banned term in a binary file's name | `test_check_boundary_terms.Mutants.test_mutant_term_in_binary_file_name_is_killed` |
| term file missing at the given path | `test_check_boundary_terms.FailClosed.test_missing_term_file_fails_closed` |
| term file missing at the default path | `test_check_boundary_terms.FailClosed.test_default_term_path_missing_fails_closed` |
| term file present but empty after comments | `test_check_boundary_terms.FailClosed.test_empty_term_file_fails_closed` |
| term file unreadable | `test_check_boundary_terms.FailClosed.test_unreadable_term_file_fails_closed` |

### review-refs

| Mutant | Killed by |
| --- | --- |
| a reference into a documentation root the tree does not carry | `test_check_review_refs.Mutants.test_mutant_orphaned_reference_is_killed` |
| a reference resolving to a path git ignores | `test_check_review_refs.Mutants.test_mutant_reference_to_untracked_path_is_killed` |
| a whitespace-only reference entry | `test_check_review_refs.Mutants.test_mutant_empty_reference_entry_is_killed` |
| an empty `review_refs` array | `test_check_review_refs.Mutants.test_mutant_empty_reference_array_is_killed` |
| an absolute-path reference | `test_check_review_refs.Mutants.test_mutant_absolute_reference_is_killed` |
| a reference escaping the repository root | `test_check_review_refs.Mutants.test_mutant_escaping_reference_is_killed` |
| a non-string reference entry | `test_check_review_refs.Mutants.test_mutant_non_string_reference_is_killed` |
| a carried `.json` file that does not parse | `test_check_review_refs.Mutants.test_mutant_unparseable_json_is_killed` |

### hostnames

| Mutant | Killed by |
| --- | --- |
| a live external host hardcoded in a carried client test | `test_check_hostnames.Mutants.test_mutant_live_external_hostname_is_killed` |
| a host reachable only through the URI-authority rule | `test_check_hostnames.Mutants.test_mutant_url_with_unusual_tld_is_killed` |
| a host recognized by its `:port` suffix | `test_check_hostnames.Mutants.test_mutant_host_with_port_is_killed` |
| a non-loopback, non-documentation IPv4 literal | `test_check_hostnames.Mutants.test_mutant_public_ipv4_address_is_killed` |
| a second host, unexcused by another host's allowlist entry | `test_check_hostnames.Mutants.test_mutant_survives_a_narrower_allowlist` |
| a bare host in source under a surviving TLD — tier 3 still fires after the identifier trim | `test_check_hostnames.Mutants.test_mutant_bare_host_on_a_plausible_tld_is_killed` |
| a real host under a label tier 3 gave up — tier 1 covers it, so the trim opened no hole | `test_check_hostnames.Mutants.test_mutant_excluded_label_still_dies_in_url_form` |
| allowlist missing | `test_check_hostnames.FailClosed.test_missing_allowlist_fails_closed` |
| allowlist empty after comments | `test_check_hostnames.FailClosed.test_entry_less_allowlist_fails_closed` |
| allowlist unreadable | `test_check_hostnames.FailClosed.test_unreadable_allowlist_fails_closed` |

### clean-room

| Mutant | Killed by |
| --- | --- |
| a tool reading a path under the private research root | `test_check_clean_room.Mutants.test_mutant_load_bearing_private_root_reference_is_killed` |
| content pointing at the ignored placeholder root | `test_check_clean_room.Mutants.test_mutant_placeholder_root_reference_is_killed` |
| a private root present on disk rather than absent | `test_check_clean_room.Mutants.test_mutant_private_root_present_on_disk_is_killed` |
| a private root that git would let you commit | `test_check_clean_room.Mutants.test_mutant_unignored_private_root_is_killed` |
| a stale allowlist entry exempting a file that no longer exists | `test_check_clean_room.Mutants.test_mutant_stale_allowlist_entry_is_killed` |
| allowlist missing | `test_check_clean_room.FailClosed.test_missing_allowlist_fails_closed` |
| allowlist empty after comments | `test_check_clean_room.FailClosed.test_entry_less_allowlist_fails_closed` |
| allowlist unreadable | `test_check_clean_room.FailClosed.test_unreadable_allowlist_fails_closed` |

### markdown-links

| Mutant | Killed by |
| --- | --- |
| a link to a document the tree does not carry | `test_check_markdown_links.Mutants.test_mutant_dead_link_is_killed` |
| a `#fragment` matching no heading in the target | `test_check_markdown_links.Mutants.test_mutant_dead_anchor_is_killed` |
| a link to a real file git ignores | `test_check_markdown_links.Mutants.test_mutant_link_to_an_ignored_file_is_killed` |
| a link escaping the repository root | `test_check_markdown_links.Mutants.test_mutant_link_escaping_the_repository_is_killed` |
| an absolute-path link target | `test_check_markdown_links.Mutants.test_mutant_absolute_link_is_killed` |
| a dead reference-style link definition | `test_check_markdown_links.Mutants.test_mutant_dead_reference_definition_is_killed` |
| a dead same-file anchor | `test_check_markdown_links.Mutants.test_mutant_dead_same_file_anchor_is_killed` |

Two false-positive classes are guarded as deliberately as the mutants, because a
link check that cries wolf gets disabled: `SlugRule` pins GitHub's per-space
hyphenation (`observer / debug` becomes `observer--debug`, not `observer-debug`)
and duplicate-heading numbering, and `CompliantTree` covers fenced code, inline
code spans, directory targets, explicit HTML anchors, and external schemes.
External `http(s)` links are **out of scope**: a check whose result depends on
somebody else's uptime is a check that fails for reasons this tree cannot fix.

### step-targets

| Mutant | Killed by |
| --- | --- |
| a step naming a test module that does not exist | `test_verification_targets.Mutants.test_mutant_step_naming_a_missing_module_is_killed` |
| a step naming a script that does not exist | `test_verification_targets.Mutants.test_mutant_step_naming_a_missing_script_is_killed` |
| a step naming a client resource that does not exist | `test_verification_targets.Mutants.test_mutant_step_naming_a_missing_client_script_is_killed` |
| a step requiring a capability nobody defined | `test_verification_targets.Mutants.test_mutant_step_with_an_unknown_capability_is_killed` |
| a step owned by a scope that does not exist | `test_verification_targets.Mutants.test_mutant_step_owned_by_a_scope_that_does_not_exist_is_killed` |
| a missing target reachable only through a step's degraded form | `test_verification_targets.Mutants.test_mutant_in_a_degraded_argv_is_killed_too` |
| a `tests/test_*.py` module no owner class claims | `test_verification_table.PythonOwnership.test_an_unclassified_module_fails_closed` |
| a classified module that has been deleted | `test_verification_table.PythonOwnership.test_a_classified_module_that_vanished_fails_closed` |
| a module classified by two owners at once | `test_verification_table.PythonOwnership.test_a_module_classified_twice_fails_closed` |
| a fast-lane plan smuggling in an expensive scope nothing changed | `test_verification_resolve.TheExclusionRule.test_an_expensive_scope_without_a_cause_is_refused` |
| a fast-lane plan selecting the owner-invoked capture lane | `test_verification_resolve.TheExclusionRule.test_the_owner_invoked_lane_is_refused_in_a_fast_plan` |

The last four are the same defect class as the others — the single source of
truth for verification being wrong about itself — so they are qualified here
rather than left as ordinary tests.

### The shared scan surface

`tools/boundary_common.py` defines the file set all four checks stand on. If it
is wrong, all four are wrong in the same direction and silently, so it is proven
directly rather than by implication: `test_boundary_common` covers the carried
set, the binary sniff, the list-file parser, and the ignore probe.

## Adding a check

1. Write it against `tools/boundary_common.py` so it scans the same file set and
   speaks the same exit codes.
2. Give it a fail-closed path for every input it depends on, and a test per
   fail-closed case.
3. Build at least one deliberate mutant per defect class it claims to catch, in
   a temporary repository, and prove the check kills it.
4. Add its rows to the table above. Until those rows exist, the check is
   advisory: run it, report it, do not block on it.
5. Add it to `CHECKS` in `tools/run_checks.py`. That is the registry, and the
   verification runner reads it — there is no second place to add it, and
   forgetting this step is the only way to write a check that never runs.
