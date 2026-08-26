#!/usr/bin/env python3
"""Fail when a carried file names a real external host that is not allowlisted.

What this defends
-----------------
The predecessor tree hardcoded one live external hostname ten times in a
tracked client test while every other fixture used reserved `.invalid` names.
A real host inside a public tree is an operational fact leaking out of a code
artifact: it names infrastructure, it invites traffic, and it rots. Tests and
fixtures get reserved names; anything else earns an allowlist line with a
reason.

Allowed without an allowlist entry
----------------------------------
* `localhost` and any `*.localhost` name (RFC 6761).
* Any name under the reserved TLDs `.invalid`, `.test`, `.example`, and
  `.localhost` (RFC 2606 / RFC 6761).
* `example.com`, `example.net`, `example.org` and anything beneath them.
* Loopback IPv4 literals (`127.0.0.0/8`), the unspecified address `0.0.0.0`,
  and the RFC 5737 documentation ranges `192.0.2.0/24`, `198.51.100.0/24`, and
  `203.0.113.0/24` — the address-literal equivalent of `example.com`.

Everything else must appear in the allowlist file, one host per line, with a
`#` comment giving the reason it is legitimate.

Detection rule
--------------
Candidates are collected from every carried text file, in three tiers of
decreasing confidence. A candidate whose final label has no letter in it is
left to the IPv4 rule — a TLD is alphabetic.

1. **Named authority** — `scheme://host` or `userinfo@host`. The syntax itself
   says "host", so the candidate is flagged whatever its final label is. This
   tier is what closes the gap tier 3 opens: a real host under an unusual TLD
   reaches a tree as a URL.
2. **Ambiguous authority** — `//host` without a scheme, or `host:<port>`. Both
   collide with ordinary source text (`//notes.md` is a comment,
   `module.py:31` is a source reference), so a candidate whose final label is
   in `COMMON_FILE_EXTENSIONS` is dropped.
3. **Bare dotted names**, which are otherwise indistinguishable from filenames
   (`notes.txt`, `world.tscn`) and from field access (`engine.world`,
   `item.name`). Only a final label in `PLAUSIBLE_TLDS` makes one a candidate.
   Two kinds of label are deliberately absent from that set: this project's
   source-file extensions, and ordinary code-identifier vocabulary
   (`EXCLUDED_IDENTIFIER_LABELS`). A real host under an excluded label still
   dies in tier 1 or tier 2.

**Known limitation.** Tier 3 discriminates by label, which is a proxy for the
real question: is this dotted name a string a human wrote, or an expression the
compiler reads? The sharper rule is to fire tier 3 only inside quoted strings
and non-source files. That would need its own mutants and its own owner
decision, so it is recorded in `docs/boundary-checks.md` rather than done
quietly here.

Fail-closed semantics
---------------------
A missing, unreadable, or entry-less allowlist file exits 3. The allowlist is
tracked configuration; its absence means the tree is not in the shape this
check was written against, and guessing is not an option a boundary check has.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from boundary_common import (  # noqa: E402
    ConfigError,
    Finding,
    carried_files,
    fail_closed,
    load_list_file,
    read_text,
    report,
    resolve_root,
)

CHECK_NAME = "hostnames"
DEFAULT_ALLOWLIST_PATH = "tools/hostname-allowlist.txt"

RESERVED_TLDS = frozenset({"invalid", "test", "example", "localhost"})
RESERVED_DOMAINS = frozenset({"example.com", "example.net", "example.org"})

# RFC 5737 documentation ranges, as /24 prefixes.
DOCUMENTATION_IPV4_PREFIXES = ("192.0.2.", "198.51.100.", "203.0.113.")

# Final labels that make a bare dotted name worth treating as a host.
#
# Two kinds of label are deliberately absent. First, this project's source-file
# extensions (rs, py, sh, so, md, gd, js, ts, in, ac, am, pl, pm, ...), so
# ordinary filenames do not fire. Second, and by the same principle, labels that
# are ordinary CODE-IDENTIFIER vocabulary: a tier-3 candidate is just a dotted
# name, and `engine.world` or `item.name` is field access, not a host. See
# EXCLUDED_IDENTIFIER_LABELS below for the list and the evidence.
#
# Giving a label up here costs only tier 3. A real host under one of these TLDs
# still dies in tier 1 or tier 2, where the syntax says "host" out loud.
EXCLUDED_IDENTIFIER_LABELS = frozenset(
    {
        # Measured against the ported rules, protocol, and authoring crates:
        # these produced 894 field-access false positives and zero true hits.
        "world",  # 799 — engine.world, self.world, parts.world
        "name",   # 46  — item.name, actor.name
        "site",   # 36  — origin.site, actor.location.site
        "at",     # 8   — edge.at, left.at, right.at
        "live",   # 5   — self.live
        # Not yet observed here, but the same class and ubiquitous in code
        # (self.info, log.info). Excluded before it costs another round.
        "info",
        # Measured against ported client: ConnectionStateMachine.ONLINE (enum),
        # file.store_string (GDScript FileAccess method).
        "online",
        "store",
        # Measured when the Workbench's staged operations arrived: 3 —
        # contracts.no_executor, a module attribute; and 2 — arguments.click,
        # an argparse attribute in a tool whose whole subject is pointing at
        # things. Both are ordinary code-identifier vocabulary in this tree.
        "no",
        "click",
    }
)

_TLD_VOCABULARY = frozenset(
    {
        "com", "net", "org", "io", "dev", "app", "co", "gov", "edu", "mil",
        "int", "info", "biz", "name", "pro", "xyz", "cloud", "site", "online",
        "tech", "live", "shop", "store", "blog", "wiki", "news", "media",
        "zone", "world", "today", "link", "click", "email", "games", "gg",
        "tv", "me", "us", "uk", "ca", "de", "fr", "jp", "cn", "ru", "au",
        "nz", "br", "mx", "es", "it", "nl", "se", "no", "fi", "dk", "ch",
        "at", "be", "pt", "gr", "cz", "ie", "il", "za", "kr", "tw", "hk",
        "sg", "eu",
    }
)

# The exclusion is applied by construction, not by hand-deleting entries above,
# so re-adding a label to the vocabulary cannot silently undo it.
PLAUSIBLE_TLDS = (_TLD_VOCABULARY | RESERVED_TLDS) - EXCLUDED_IDENTIFIER_LABELS

# Final labels that mean "this is a filename, not a host". Consulted only where
# the surrounding syntax is ambiguous — `module.py:31` is a source reference,
# not a host and port. A URL says what it is and is never filtered this way.
COMMON_FILE_EXTENSIONS = frozenset(
    {
        "py", "rs", "gd", "js", "ts", "json", "toml", "yml", "yaml", "md",
        "txt", "sh", "bash", "so", "a", "o", "tscn", "tres", "png", "jpg",
        "jpeg", "gif", "svg", "webp", "css", "html", "htm", "lock", "cfg",
        "ini", "conf", "csv", "tsv", "log", "gz", "zip", "tar", "exe", "dll",
        "rb", "go", "c", "h", "cpp", "hpp", "java", "kt", "sql", "xml", "bin",
        "dat", "cs", "pl", "pm", "lua", "bat", "ps1", "pdf", "wav", "ogg",
        "mp3", "ttf", "otf", "glb", "gltf", "obj", "blend",
    }
)

_LABEL = r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?"
_DOTTED = rf"{_LABEL}(?:\.{_LABEL})+"

# Unambiguous authority positions: a scheme's URI authority, or userinfo@host.
# Both say "host" in their own syntax, so no label filtering applies to them.
_SCHEME_AUTHORITY = re.compile(rf"[A-Za-z][A-Za-z0-9+.-]*://(?P<host>{_DOTTED})")
_USERINFO_AUTHORITY = re.compile(rf"[A-Za-z0-9._%+-]+@(?P<host>{_DOTTED})")
# A scheme-relative authority is ambiguous with a C-style comment (`//notes.md`),
# so extensions are filtered here as they are for the port form.
_SCHEMELESS_AUTHORITY = re.compile(rf"(?<![A-Za-z0-9:])//(?P<host>{_DOTTED})")
# Ambiguous with a source reference (`module.py:31`), so extensions are filtered.
_PORT_HOST = re.compile(
    rf"(?<![A-Za-z0-9._:/-])(?P<host>{_DOTTED})(?=:\d{{1,5}}(?![0-9]))"
)
# Ambiguous with any filename, so only a plausible TLD makes it a candidate.
_BARE_HOST = re.compile(rf"(?<![A-Za-z0-9.@/_-])(?P<host>{_DOTTED})(?![A-Za-z0-9-])")
_IPV4_PATTERN = re.compile(r"(?<![A-Za-z0-9._:-])(\d{1,3}(?:\.\d{1,3}){3})(?![0-9.])")


def _is_allowed_host(host: str, allowlist: set[str]) -> bool:
    lowered = host.lower().rstrip(".")
    if lowered in allowlist:
        return True
    labels = lowered.split(".")
    if labels[-1] in RESERVED_TLDS:
        return True
    for index in range(len(labels)):
        if ".".join(labels[index:]) in RESERVED_DOMAINS:
            return True
    return False


def _is_allowed_ipv4(address: str, allowlist: set[str]) -> bool:
    octets = address.split(".")
    if any(not octet.isdigit() or int(octet) > 255 for octet in octets):
        return True  # not a valid dotted quad; not an address at all
    if address in allowlist:
        return True
    if octets[0] == "127" or address == "0.0.0.0":
        return True
    return any(address.startswith(prefix) for prefix in DOCUMENTATION_IPV4_PREFIXES)


def _candidates(line: str) -> list[str]:
    """Collect the hostname candidates on one line, per the three-branch rule."""
    hosts: list[str] = []

    def offer(host: str, gate) -> None:
        final_label = host.rsplit(".", 1)[-1].lower()
        # A TLD is alphabetic. An all-digit final label means a dotted quad,
        # which the IPv4 rule owns.
        if not any(character.isalpha() for character in final_label):
            return
        if gate(final_label):
            hosts.append(host)

    def anything(label: str) -> bool:
        return True

    def not_a_filename(label: str) -> bool:
        return label not in COMMON_FILE_EXTENSIONS

    for pattern in (_SCHEME_AUTHORITY, _USERINFO_AUTHORITY):
        for match in pattern.finditer(line):
            offer(match.group("host"), anything)
    for pattern in (_SCHEMELESS_AUTHORITY, _PORT_HOST):
        for match in pattern.finditer(line):
            offer(match.group("host"), not_a_filename)
    for match in _BARE_HOST.finditer(line):
        offer(match.group("host"), lambda label: label in PLAUSIBLE_TLDS)
    return hosts


def scan(root: Path, allowlist_path: Path) -> list[Finding]:
    allowlist = {entry.lower() for entry in load_list_file(allowlist_path, "hostname-allowlist")}
    findings: list[Finding] = []

    for relative in carried_files(root):
        text = read_text(root / relative)
        if text is None:
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            seen: set[str] = set()
            for host in _candidates(line):
                lowered = host.lower()
                if lowered in seen:
                    continue
                seen.add(lowered)
                if _is_allowed_host(host, allowlist):
                    continue
                findings.append(
                    Finding(relative, f"non-reserved hostname {host!r}", line=number)
                )
            for match in _IPV4_PATTERN.finditer(line):
                address = match.group(1)
                if _is_allowed_ipv4(address, allowlist):
                    continue
                findings.append(
                    Finding(relative, f"non-loopback IPv4 address {address!r}", line=number)
                )
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root to scan")
    parser.add_argument(
        "--allowlist",
        default=None,
        help=f"allowlist data file (default: <root>/{DEFAULT_ALLOWLIST_PATH})",
    )
    arguments = parser.parse_args(argv)

    try:
        root = resolve_root(arguments.root)
        allowlist_path = (
            Path(arguments.allowlist).resolve()
            if arguments.allowlist
            else root / DEFAULT_ALLOWLIST_PATH
        )
        findings = scan(root, allowlist_path)
    except ConfigError as error:
        return fail_closed(CHECK_NAME, error)
    return report(CHECK_NAME, findings)


if __name__ == "__main__":
    sys.exit(main())
