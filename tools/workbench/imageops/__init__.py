"""The provider-neutral image-operation layer, and the preservation rule it enforces.

Five operations (`contracts.py`), one mask format over two address spaces
(`masks.py`), a standard-library PNG codec (`png.py`), one reference adapter
behind an injectable registry (`adapters.py`), the project-owned compositing step
(`preserve.py`), and one executor (`run.py`).

The rule the whole package exists for: **only the exact commit mask may replace
accepted source pixels, and everything outside it is restored from the source by
this project's own code after the adapter returns.** A model may be handed a
generous context image, because context is what makes an edit blend; the context
is not permission. Expanding the commit boundary is an explicit owner act,
recorded as its own operation. It is enforced here rather than requested of a
provider because no adapter can be trusted to enforce it — and an adapter that
was trusted and then ignored the boundary would erode accepted work one edit at a
time, invisibly.

Adapters are never architecture authority. An adapter does not define the
operation set, does not own the candidate lifecycle, does not decide what gets
promoted, and can be replaced without touching anything above it. What an adapter
needs beyond the shared fields rides in its own typed block.

**Four hard limits, all current facts about this tree:**

1. **No hosted adapter.** The registry holds one local adapter.
2. **No generative model.** Nothing here calls one, and adding an AI runtime of
   any kind is an owner decision (`AGENTS.md`).
3. **No network.** Nothing in this package opens a socket.
4. **No process.** Nothing here starts a program; `tests/test_workbench_loop.py`
   parses every module in the package and fails if that stops being true.
"""

from __future__ import annotations

from .adapters import REGISTRY, Adapter, AdapterRefused, lookup, palette_fill
from .contracts import (
    CONTRACTS,
    VOCABULARY,
    AdapterBlock,
    AssetOperation,
    Context,
    OperationContract,
    OperationRefused,
    SourceRef,
    contract,
    no_executor,
    validate,
)
from .masks import Mask, MaskUnreadable, decode_p1, read_mask, write_mask
from .png import Image, ImageUnreadable, decode, encode, size
from .preserve import (
    Composited,
    PreservationRefused,
    Region,
    composite,
    context_region,
    crop,
    preserved_outside,
)
from .run import EditRefused, EditResult, execute, run_edit_region

__all__ = [
    "REGISTRY",
    "Adapter",
    "AdapterBlock",
    "AdapterRefused",
    "AssetOperation",
    "CONTRACTS",
    "Composited",
    "Context",
    "EditRefused",
    "EditResult",
    "Image",
    "ImageUnreadable",
    "Mask",
    "MaskUnreadable",
    "OperationContract",
    "OperationRefused",
    "PreservationRefused",
    "Region",
    "SourceRef",
    "VOCABULARY",
    "composite",
    "context_region",
    "contract",
    "crop",
    "decode",
    "decode_p1",
    "encode",
    "execute",
    "lookup",
    "no_executor",
    "palette_fill",
    "preserved_outside",
    "read_mask",
    "run_edit_region",
    "size",
    "validate",
    "write_mask",
]
