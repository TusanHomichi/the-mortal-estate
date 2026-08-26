"""Semantic resolution: what occupies the cells the owner pointed at.

There is exactly one implementation of this, and both consumers reach it — the
local server answering the browser, and `resolve.py` answering an agent. Agent
parity is a law, and two resolvers would be two chances to disagree about what
"here" means.

**Ranking is a convenience. Ambiguity is data.** When several identities
overlap a selection the packet carries all of them, ordered, with an explicit
flag; no consumer picks. The ordering exists so a human reading the packet sees
the likely answer first, not so a program can take the first entry and skip the
question.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .projection import Member

#: Identity kinds that name something a selection could be *about*, as opposed
#: to describing the ground it sits on. Only these participate in the ambiguity
#: rule: every cell has terrain, so naming terrain is description, not a choice.
OCCUPANT_KINDS = ("structure", "transition", "landmark", "route")

#: Tie-break order when two identities account for the same share of the
#: selection. Authored features outrank the ground they stand on.
KIND_ORDER = ("structure", "transition", "landmark", "route", "cell_terrain", "layer")

BASE_TERRAIN_LAYER = "base_terrain"


@dataclass
class Identity:
    """One thing that occupies part of a selection, and how much of it."""

    kind: str
    identity: str
    cells: tuple[tuple[int, int], ...]
    identity_cells: int
    detail: dict = field(default_factory=dict)

    def coverage(self, selection_size: int) -> dict:
        return {
            "covered_cells": len(self.cells),
            "selection_cells": selection_size,
            "identity_cells": self.identity_cells,
            "selection_coverage": _ratio(len(self.cells), selection_size),
            "identity_coverage": _ratio(len(self.cells), self.identity_cells),
        }

    def as_record(self, rank: int, selection_size: int) -> dict:
        return {
            "rank": rank,
            "kind": self.kind,
            "identity": self.identity,
            "cells": [{"x": x, "y": y} for x, y in self.cells],
            "coverage": self.coverage(selection_size),
            "detail": self.detail,
        }


def _ratio(part: int, whole: int) -> float:
    """A coverage ratio rounded to six places so two runs never differ in noise."""
    if whole <= 0:
        return 0.0
    return round(part / whole, 6)


def _ordered(cells) -> tuple[tuple[int, int], ...]:
    return tuple(sorted(cells, key=lambda cell: (cell[1], cell[0])))


def _point(value: dict) -> tuple[int, int]:
    return int(value["x"]), int(value["y"])


def _footprint(structure: dict) -> set[tuple[int, int]]:
    origin_x, origin_y = int(structure["x"]), int(structure["y"])
    return {
        (origin_x + dx, origin_y + dy)
        for dy in range(int(structure["height"]))
        for dx in range(int(structure["width"]))
    }


def _terrain_identities(member: Member, cells) -> list[Identity]:
    """One identity per distinct terrain class the selection covers.

    Per class rather than per cell: a box over forty grass cells means "grass",
    and forty identical identities would bury the structure standing in it.
    """
    covered: dict[tuple[str, str], list[tuple[int, int]]] = {}
    for cell in cells:
        for entry in member.terrain(cell):
            covered.setdefault((entry["class"], entry["layer"]), []).append(cell)

    totals: dict[tuple[str, str], int] = {}
    for record in member.cells.values():
        for entry in record["terrain"]:
            key = (entry["class"], entry["layer"])
            totals[key] = totals.get(key, 0) + 1

    identities = []
    for (terrain_class, layer), owned in sorted(covered.items()):
        ordered = _ordered(owned)
        identities.append(
            Identity(
                kind="cell_terrain",
                identity=f"terrain:{member.member}:{terrain_class}",
                cells=ordered,
                identity_cells=totals[(terrain_class, layer)],
                detail={
                    "terrain_class": terrain_class,
                    "authored_layer": layer,
                    "passable_cells": sum(1 for cell in ordered if member.is_passable(cell)),
                },
            )
        )
    return identities


def _layer_identities(member: Member, cells) -> list[Identity]:
    """Which authored layers the address belongs to.

    A truth edit and a dressing edit over the same pixels must stay
    distinguishable, and the layer is what distinguishes them.
    """
    covered: dict[str, set[tuple[int, int]]] = {}
    for cell in cells:
        for entry in member.terrain(cell):
            covered.setdefault(entry["layer"], set()).add(cell)

    totals: dict[str, set[tuple[int, int]]] = {}
    for cell, record in member.cells.items():
        for entry in record["terrain"]:
            totals.setdefault(entry["layer"], set()).add(cell)

    return [
        Identity(
            kind="layer",
            identity=f"layer:{member.member}:{layer}",
            cells=_ordered(owned),
            identity_cells=len(totals[layer]),
            detail={"authored_layer": layer, "member": member.member},
        )
        for layer, owned in sorted(covered.items())
    ]


def _structure_identities(member: Member, cells) -> list[Identity]:
    selection = set(cells)
    identities = []
    for structure in member.structures:
        footprint = _footprint(structure)
        access = _point(structure["access"])
        door = _point(structure["facade_door"])
        owned = footprint | {access}
        covered = owned & selection
        if not covered:
            continue
        identities.append(
            Identity(
                kind="structure",
                identity=f"structure:{member.member}:{structure['id']}",
                cells=_ordered(covered),
                identity_cells=len(owned),
                detail={
                    "structure_id": structure["id"],
                    "purpose": structure["purpose"],
                    "scope": structure["scope"],
                    "footprint": {
                        "x": int(structure["x"]),
                        "y": int(structure["y"]),
                        "width": int(structure["width"]),
                        "height": int(structure["height"]),
                    },
                    "access_cell": {"x": access[0], "y": access[1]},
                    "facade_door": {"x": door[0], "y": door[1]},
                    "footprint_cells_covered": len(footprint & selection),
                    "access_cell_covered": access in selection,
                    "facade_door_covered": door in selection,
                },
            )
        )
    return identities


def _landmark_identities(member: Member, cells) -> list[Identity]:
    selection = set(cells)
    return [
        Identity(
            kind="landmark",
            identity=f"landmark:{member.member}:{landmark['id']}",
            cells=(_point(landmark["at"]),),
            identity_cells=1,
            detail={
                "landmark_id": landmark["id"],
                "role": landmark["role"],
                "at": dict(landmark["at"]),
            },
        )
        for landmark in member.landmarks
        if _point(landmark["at"]) in selection
    ]


def _transition_identities(member: Member, cells) -> list[Identity]:
    selection = set(cells)
    identities = []
    for transition in member.transitions:
        marker = _point(transition["marker"])
        access = _point(transition["access"])
        covered = {marker, access} & selection
        if not covered:
            continue
        identities.append(
            Identity(
                kind="transition",
                identity=f"transition:{member.member}:{transition['id']}",
                cells=_ordered(covered),
                identity_cells=2,
                detail={
                    "transition_id": transition["id"],
                    "member": transition["member"],
                    "target_member": transition["target_member"],
                    "paired_transition": transition["paired_transition"],
                    "direction": transition["direction"],
                    "marker": dict(transition["marker"]),
                    "access_cell": dict(transition["access"]),
                    "marker_covered": marker in selection,
                    "access_cell_covered": access in selection,
                },
            )
        )
    return identities


def _route_identities(member: Member, cells) -> list[Identity]:
    """Route membership for the covered cells.

    The authored routes layer carries no per-route identity, so this names
    membership rather than inventing a route id the master does not have.
    """
    covered = set(cells) & member.routes
    if not covered:
        return []
    return [
        Identity(
            kind="route",
            identity=f"route:{member.member}",
            cells=_ordered(covered),
            identity_cells=len(member.routes),
            detail={
                "member": member.member,
                "route_cells_in_member": len(member.routes),
            },
        )
    ]


def _rank_key(identity: Identity, selection_size: int):
    coverage = identity.coverage(selection_size)
    return (
        -coverage["selection_coverage"],
        -coverage["identity_coverage"],
        KIND_ORDER.index(identity.kind),
        identity.identity,
    )


def is_ambiguous(identities: list[Identity], selection_size: int) -> bool:
    """Whether a consumer must ask rather than act.

    The rule, stated once so it can be argued with:

    1. more than one occupant identity in the selection — two structures, or a
       landmark standing on a route — is ambiguous;
    2. exactly one occupant that does not account for the whole selection is
       ambiguous, because the rest of what was pointed at is unexplained;
    3. no occupant at all is ambiguous only if the selection spans more than one
       base terrain class.

    It errs toward asking. Over-flagging costs one question; under-flagging
    spends an agent's confidence on the wrong address.
    """
    occupants = [row for row in identities if row.kind in OCCUPANT_KINDS]
    if len(occupants) > 1:
        return True
    if len(occupants) == 1:
        return occupants[0].coverage(selection_size)["selection_coverage"] < 1.0
    grounds = {
        row.detail["terrain_class"]
        for row in identities
        if row.kind == "cell_terrain" and row.detail["authored_layer"] == BASE_TERRAIN_LAYER
    }
    return len(grounds) > 1


def resolve(member: Member, cells) -> dict:
    """Resolve a set of cells into the ranked identity set that occupies them."""
    ordered_cells = _ordered(set(cells))
    identities = (
        _structure_identities(member, ordered_cells)
        + _transition_identities(member, ordered_cells)
        + _landmark_identities(member, ordered_cells)
        + _route_identities(member, ordered_cells)
        + _terrain_identities(member, ordered_cells)
        + _layer_identities(member, ordered_cells)
    )
    size = len(ordered_cells)
    identities.sort(key=lambda row: _rank_key(row, size))
    records = [row.as_record(index + 1, size) for index, row in enumerate(identities)]
    ambiguous = is_ambiguous(identities, size)
    return {
        "member": member.member,
        "cells": [{"x": x, "y": y} for x, y in ordered_cells],
        "semantic": records,
        # Candidates are a VIEW of the ranked set, never a second set, so no
        # consumer has to reconcile two lists that could disagree.
        "candidates": records if ambiguous else [],
        "ambiguous": ambiguous,
    }
