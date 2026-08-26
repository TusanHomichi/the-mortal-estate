"""The compiled logical surface, and the digests that bind a selection to it.

The document this loads is written by the authoring compiler
(`crates/tme-authoring/src/export.rs`), not by anything here. That is the point:
the logical view renders the compiler's own truth, so a logical address is by
construction an address in the authoritative frame. Nothing in this module
parses an authored `.tmj`, recomputes passability, or decides what a cell means.

**Fail-closed staleness.** Every consumer — the local server, `resolve.py`, an
agent with a text editor — recomputes the bound digests before acting. A
mismatch is a refusal naming the digest that moved. There is no nearest-match
fallback and no re-resolution heuristic, because a selection that silently
follows a moved world converts a precise instruction into a confident wrong one.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

DEFAULT_PROJECTION_PATH = "content/authoring-fixture/generated/workbench_projection.json"
PROJECTION_KIND = "workbench_logical_projection"
PROJECTION_ROLE = "logical_projection"

#: A CANDIDATE's logical view, emitted by the same compiler from a document
#: nobody has attested. It carries a different kind and is loaded by a different
#: function, so the accepted view and a candidate view cannot be mistaken for
#: each other by a caller that forgot which one it asked for — `load` refuses a
#: candidate document and `load_candidate` refuses the accepted one.
CANDIDATE_KIND = "workbench_candidate_projection"
CANDIDATE_ROLE = "candidate_projection"
SCHEMA_VERSION = 1

REBUILD_COMMAND = "cargo run -p tme-authoring"


class WorkbenchError(Exception):
    """Base class for every refusal this package raises."""


class ProjectionUnavailable(WorkbenchError):
    """The logical projection is missing or is not the document it claims to be.

    Honest unavailability, never a false pass: the Workbench refuses to open
    rather than showing a view it cannot bind to real bytes.
    """


class StaleSelection(WorkbenchError):
    """A bound source no longer holds the bytes the selection was taken against."""

    def __init__(self, moved: list[dict[str, str | None]]) -> None:
        self.moved = moved
        detail = "; ".join(
            f"{entry['path']} " + (
                "is missing"
                if entry["actual"] is None
                else f"digest moved (bound {entry['expected']}, on disk {entry['actual']})"
            )
            for entry in moved
        )
        super().__init__(f"stale selection: {detail}")


def digest_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def digest_file(path: Path) -> str | None:
    """The file's SHA-256, or None when it is not there to be read."""
    try:
        return digest_bytes(path.read_bytes())
    except OSError:
        return None


@dataclass(frozen=True)
class Source:
    """One addressed file and the digest a selection was bound to."""

    role: str
    path: str
    sha256: str

    def as_record(self) -> dict[str, str]:
        return {"role": self.role, "path": self.path, "sha256": self.sha256}

    @classmethod
    def from_record(cls, record: dict) -> "Source":
        try:
            return cls(str(record["role"]), str(record["path"]), str(record["sha256"]))
        except (KeyError, TypeError) as error:
            raise ProjectionUnavailable(f"malformed source binding: {record!r}") from error


@dataclass(frozen=True)
class Member:
    """One authored member of the land, exactly as the compiler emitted it."""

    member: str
    width: int
    height: int
    cells: dict[tuple[int, int], dict]
    routes: frozenset[tuple[int, int]]
    structures: tuple[dict, ...]
    landmarks: tuple[dict, ...]
    transitions: tuple[dict, ...]

    def contains(self, cell: tuple[int, int]) -> bool:
        return cell in self.cells

    def terrain(self, cell: tuple[int, int]) -> list[dict]:
        return list(self.cells[cell]["terrain"])

    def is_passable(self, cell: tuple[int, int]) -> bool:
        return bool(self.cells[cell]["passable"])


@dataclass(frozen=True)
class Projection:
    """The loaded logical projection and its complete source binding."""

    root: Path
    path: str
    document: dict
    digest: str
    sources: tuple[Source, ...]
    members: dict[str, Member]
    land_id: str
    realm_id: str
    #: The one member the compiler will accept staged truth operations against.
    #: Declared by the land's contract and carried here, so nothing in this
    #: package holds a second opinion about which member is editable.
    candidate_member: str
    tile_size_px: int

    def member(self, name: str) -> Member:
        try:
            return self.members[name]
        except KeyError:
            raise WorkbenchError(f"the projection carries no member named {name!r}") from None

    def source_records(self) -> list[dict[str, str]]:
        return [source.as_record() for source in self.sources]


def _point(value: dict) -> tuple[int, int]:
    return int(value["x"]), int(value["y"])


def _member(record: dict) -> Member:
    cells: dict[tuple[int, int], dict] = {}
    for cell in record["cells"]:
        cells[(int(cell["x"]), int(cell["y"]))] = {
            "passable": bool(cell["passable"]),
            "terrain": [
                {"class": str(entry["class"]), "layer": str(entry["layer"])}
                for entry in cell["terrain"]
            ],
        }
    return Member(
        member=str(record["member"]),
        width=int(record["width"]),
        height=int(record["height"]),
        cells=cells,
        routes=frozenset(_point(route) for route in record["routes"]),
        structures=tuple(record["structures"]),
        landmarks=tuple(record["landmarks"]),
        transitions=tuple(record["transitions"]),
    )


def load_candidate(root: Path, path: str) -> Projection:
    """Load a candidate's logical view.

    The same shape, the same renderer, the same resolver — and its own kind, so
    that nothing which asked for the accepted land can be handed a candidate.
    Its one bound source is the candidate document itself, which lives in a
    session and is replaced by the next preview.
    """
    return load(root, path, kind=CANDIDATE_KIND, role=CANDIDATE_ROLE)


def load(
    root: Path,
    path: str = DEFAULT_PROJECTION_PATH,
    *,
    kind: str = PROJECTION_KIND,
    role: str = PROJECTION_ROLE,
) -> Projection:
    """Load the logical projection, or refuse with a reason and a repair."""
    target = Path(root) / path
    try:
        payload = target.read_bytes()
    except OSError as error:
        raise ProjectionUnavailable(
            f"the logical projection is unavailable at {path}: {error}. "
            f"Build it with: {REBUILD_COMMAND}"
        ) from error
    try:
        document = json.loads(payload)
    except json.JSONDecodeError as error:
        raise ProjectionUnavailable(f"{path} is not valid JSON: {error}") from error
    if not isinstance(document, dict):
        raise ProjectionUnavailable(f"{path} is not a projection document")
    if document.get("kind") != kind:
        raise ProjectionUnavailable(
            f"{path} declares kind {document.get('kind')!r}, not {kind!r}"
        )
    if document.get("schema_version") != SCHEMA_VERSION:
        raise ProjectionUnavailable(
            f"{path} declares schema version {document.get('schema_version')!r}, "
            f"and this Workbench reads version {SCHEMA_VERSION}"
        )

    try:
        members = {
            str(record["member"]): _member(record) for record in document["members"]
        }
        sources = tuple(Source.from_record(record) for record in document["sources"])
        land_id = str(document["land_id"])
        realm_id = str(document["realm_id"])
        candidate_member = str(document["candidate_member"])
        tile_size_px = int(document["tile_size_px"])
    except (KeyError, TypeError, ValueError) as error:
        raise ProjectionUnavailable(f"{path} is missing required content: {error}") from error

    projection_digest = digest_bytes(payload)
    return Projection(
        root=Path(root),
        path=path,
        document=document,
        digest=projection_digest,
        sources=sources + (Source(role, path, projection_digest),),
        members=members,
        land_id=land_id,
        realm_id=realm_id,
        candidate_member=candidate_member,
        tile_size_px=tile_size_px,
    )


def verify(root: Path, sources) -> None:
    """Recompute every bound digest, or refuse naming the ones that moved.

    Called by every consumer before it acts on a packet — the server on each
    request, `resolve.py` on each run. It is deliberately the cheapest possible
    check: file reads and hashes, no compiler, no build, no test machinery.
    """
    moved: list[dict[str, str | None]] = []
    for source in sources:
        actual = digest_file(Path(root) / source.path)
        if actual != source.sha256:
            moved.append(
                {"role": source.role, "path": source.path,
                 "expected": source.sha256, "actual": actual}
            )
    if moved:
        raise StaleSelection(moved)
