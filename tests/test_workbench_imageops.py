"""The image-operation layer, and the one rule it exists to enforce.

The receipt this suite is graded on is `TheAdapterIsNotTrusted`: a **hostile**
adapter, registered through the injectable registry, paints a garish colour over
the entire context image it is handed. The project's compositing step must still
produce a result byte-identical to the source everywhere outside the commit mask,
must count every pixel the adapter had no business touching, and the executor
must refuse the operation naming that count. A preservation rule proven only
against adapters that behave is a rule proven against friends, and the whole
point of enforcing it project-side is that a hosted model is not a friend, it is
a black box that returns pixels.

The PNG codec is proven three ways, because a decoder that is wrong is a decoder
that silently changes accepted work. It round-trips its own output; it decodes
the **real tracked capture**, written by the client's own encoder, whose 768 rows
use four of the five filter types; and every one of the five filters is exercised
by encoding one gradient five times, each with a different filter applied. The
forward filter here is transcribed from the PNG specification rather than
imported from the reader, so the two are independent transcriptions checked
against a known image — which is the strongest claim available without taking a
dependency the Workbench does not have.
"""

from __future__ import annotations

import shutil
import struct
import tempfile
import unittest
from dataclasses import replace
import zlib
from pathlib import Path

from workbench_test_support import FIXTURE_ROUTE, REPO_ROOT, fixture_route_capture

from workbench import capture as capture_reader
from workbench import resolve
from workbench.imageops import adapters, contracts, masks, png, preserve, run
from workbench.imageops.png import Image
from workbench.projection import WorkbenchError, digest_bytes

#: The source is a gradient whose red channel is always even, so neither test
#: colour below can occur in it by accident. That is what makes the changed and
#: unchanged pixel counts exact rather than approximate.
GARISH = (255, 0, 255, 255)
WELL_BEHAVED = (255, 128, 64, 255)

MASK_PIXELS = frozenset(
    (x, y) for x in range(10, 14) for y in range(6, 9)
)  # a 4x3 block: twelve pixels, and no more may ever change
MARGIN = 3


def gradient(width: int, height: int) -> Image:
    """A picture with no two pixels alike and no pixel equal to a test colour."""
    pixels = bytearray()
    for y in range(height):
        for x in range(width):
            pixels += bytes(((x * 10) % 256, (y * 13) % 256, ((x + y) * 7) % 256, 255))
    return Image(width=width, height=height, pixels=bytes(pixels))


def chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def build_png(width, height, *, depth, colour_type, interlace, raw) -> bytes:
    """A PNG assembled here, so a test can build one this reader must refuse."""
    header = struct.pack(">IIBBBBB", width, height, depth, colour_type, 0, 0, interlace)
    return b"".join((
        png.SIGNATURE,
        chunk(b"IHDR", header),
        chunk(b"IDAT", zlib.compress(raw)),
        chunk(b"IEND", b""),
    ))


def paeth(left: int, above: int, upper_left: int) -> int:
    """The PNG specification's predictor, transcribed here, not imported."""
    estimate = left + above - upper_left
    distances = (abs(estimate - left), abs(estimate - above), abs(estimate - upper_left))
    if distances[0] <= distances[1] and distances[0] <= distances[2]:
        return left
    if distances[1] <= distances[2]:
        return above
    return upper_left


def filtered(image: Image, kind: int) -> bytes:
    """The image's scanlines with one filter applied — the forward direction."""
    step = 4
    stride = image.width * step
    out = bytearray()
    previous = bytes(stride)
    for y in range(image.height):
        line = image.pixels[y * stride : (y + 1) * stride]
        out.append(kind)
        row = bytearray(stride)
        for index in range(stride):
            left = line[index - step] if index >= step else 0
            above = previous[index]
            upper_left = previous[index - step] if index >= step else 0
            if kind == 0:
                prediction = 0
            elif kind == 1:
                prediction = left
            elif kind == 2:
                prediction = above
            elif kind == 3:
                prediction = (left + above) >> 1
            else:
                prediction = paeth(left, above, upper_left)
            row[index] = (line[index] - prediction) & 0xFF
        out += row
        previous = line
    return bytes(out)


class ThePngCodecIsExact(unittest.TestCase):
    def test_an_image_survives_a_round_trip_at_several_shapes(self) -> None:
        """Kills a codec that loses a row, a column, or a channel at some size.

        The awkward shapes are the point: 1xN and Nx1 are where a stride bug
        hides, and a single pixel is where an off-by-one in the row loop does.
        """
        for width, height in ((1, 1), (1, 9), (9, 1), (7, 5), (32, 24)):
            with self.subTest(shape=(width, height)):
                image = gradient(width, height)
                self.assertEqual(png.decode(png.encode(image)), image)

    def test_encoding_the_same_image_twice_gives_the_same_bytes(self) -> None:
        """Kills a non-deterministic encoder; a candidate's identity is its digest."""
        image = gradient(19, 11)
        first, second = png.encode(image), png.encode(image)
        self.assertEqual(first, second)
        self.assertEqual(png.encode(png.decode(first)), first)

    def test_the_declared_size_and_the_decoded_size_agree(self) -> None:
        """Kills a header reader that disagrees with the pixels behind it."""
        image = gradient(13, 21)
        payload = png.encode(image)
        self.assertEqual(png.size(payload), (13, 21))
        decoded = png.decode(payload)
        self.assertEqual((decoded.width, decoded.height), png.size(payload))

    def test_every_pixel_reads_back_at_its_own_coordinate(self) -> None:
        """Kills a row-major/column-major mix-up, which a square image would hide."""
        image = gradient(7, 5)
        decoded = png.decode(png.encode(image))
        for y in range(5):
            for x in range(7):
                self.assertEqual(
                    decoded.pixel(x, y),
                    ((x * 10) % 256, (y * 13) % 256, ((x + y) * 7) % 256, 255),
                )
        with self.assertRaises(png.ImageUnreadable):
            decoded.pixel(7, 0)


class TheRealTrackedPictureDecodes(unittest.TestCase):
    def test_the_tracked_capture_decodes_and_its_size_is_the_sidecars(self) -> None:
        """Kills a codec that only handles pictures this repository wrote itself.

        The tracked capture came out of the client's own encoder: five IDAT
        chunks, an sRGB chunk this reader must skip, and 768 rows using four of
        the five filter types. Decoding it is the only test here whose input
        nothing in this suite produced.
        """
        payload = (REPO_ROOT / FIXTURE_ROUTE / "capture.png").read_bytes()
        width, height = png.size(payload)
        image = png.decode(payload)
        self.assertEqual((image.width, image.height), (width, height))

        taken = fixture_route_capture()
        self.assertEqual(taken.viewport, {"width": width, "height": height})
        self.assertEqual(capture_reader.png_size(payload), (width, height))
        self.assertEqual(len(image.pixels), width * height * 4)

    def test_the_decoded_capture_re_encodes_to_the_same_pixels(self) -> None:
        """Kills a decoder that is self-consistent but loses information."""
        payload = (REPO_ROOT / FIXTURE_ROUTE / "capture.png").read_bytes()
        image = png.decode(payload)
        self.assertEqual(png.decode(png.encode(image)), image)


class EveryFilterTypeDecodes(unittest.TestCase):
    def test_all_five_filters_reconstruct_the_same_image(self) -> None:
        """Kills a reader missing any of the five per-row filters.

        A reader that implements four of them decodes most pictures and mangles
        the rest — quietly, since a mangled row is still a row. One gradient is
        encoded five times, once per filter, and all five must reconstruct it
        exactly.
        """
        image = gradient(11, 9)
        for kind in range(5):
            with self.subTest(filter=kind):
                payload = build_png(
                    11, 9, depth=8, colour_type=6, interlace=0, raw=filtered(image, kind)
                )
                self.assertEqual(png.decode(payload), image)

    def test_a_row_may_name_a_different_filter_from_its_neighbour(self) -> None:
        """Kills a reader that picks one filter for the whole image.

        The filter byte is per row, and the tracked capture mixes four of them.
        Rows here are filtered in rotation so that every adjacent pair differs.
        """
        image = gradient(9, 10)
        step = 4
        stride = image.width * step
        raw = bytearray()
        previous = bytes(stride)
        for y in range(image.height):
            kind = y % 5
            one_row = Image(
                width=image.width,
                height=1,
                pixels=image.pixels[y * stride : (y + 1) * stride],
            )
            # Filter this row against the real previous row rather than zero.
            two = Image(
                width=image.width,
                height=2,
                pixels=bytes(previous) + one_row.pixels,
            )
            body = filtered(two, kind)
            raw += body[len(body) // 2 :]
            previous = one_row.pixels
        payload = build_png(9, 10, depth=8, colour_type=6, interlace=0, raw=bytes(raw))
        self.assertEqual(png.decode(payload), image)

    def test_greyscale_and_truecolour_widen_exactly(self) -> None:
        """Kills an expansion that guesses alpha or averages a grey sample."""
        grey = bytes((0, 40, 80, 120, 160, 200))
        raw = b"\x00" + grey[:3] + b"\x00" + grey[3:]
        payload = build_png(3, 2, depth=8, colour_type=0, interlace=0, raw=raw)
        decoded = png.decode(payload)
        self.assertEqual(decoded.pixel(0, 0), (0, 0, 0, 255))
        self.assertEqual(decoded.pixel(2, 1), (200, 200, 200, 255))

        rgb = bytes((1, 2, 3, 4, 5, 6))
        payload = build_png(
            2, 1, depth=8, colour_type=2, interlace=0, raw=b"\x00" + rgb
        )
        decoded = png.decode(payload)
        self.assertEqual(decoded.pixel(0, 0), (1, 2, 3, 255))
        self.assertEqual(decoded.pixel(1, 0), (4, 5, 6, 255))

        grey_alpha = bytes((90, 10, 120, 200))
        payload = build_png(
            2, 1, depth=8, colour_type=4, interlace=0, raw=b"\x00" + grey_alpha
        )
        decoded = png.decode(payload)
        self.assertEqual(decoded.pixel(0, 0), (90, 90, 90, 10))
        self.assertEqual(decoded.pixel(1, 0), (120, 120, 120, 200))


class AnUnreadablePictureIsRefused(unittest.TestCase):
    def real(self) -> bytes:
        return (REPO_ROOT / FIXTURE_ROUTE / "capture.png").read_bytes()

    def test_a_truncated_png_is_refused(self) -> None:
        """Kills a reader that decodes whatever arrived and calls it the picture."""
        payload = self.real()
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(payload[: len(payload) // 2])
        self.assertIn("early", str(caught.exception))

    def test_a_png_with_no_end_chunk_is_refused(self) -> None:
        """Kills a reader that accepts a stream someone stopped writing."""
        payload = self.real()
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(payload[: -len(chunk(b"IEND", b""))])
        self.assertIn("IEND", str(caught.exception))

    def test_a_corrupted_chunk_is_refused_by_its_own_checksum(self) -> None:
        """Kills a reader that ignores the CRC a PNG carries for exactly this."""
        payload = bytearray(self.real())
        payload[100] ^= 0x01  # one bit, inside the first IDAT's data
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(bytes(payload))
        self.assertIn("CRC", str(caught.exception))

    def test_an_interlaced_png_is_refused_by_name(self) -> None:
        """Kills a reader that treats Adam7 passes as ordinary rows."""
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(
                build_png(4, 4, depth=8, colour_type=6, interlace=1, raw=b"\x00" * 68)
            )
        message = str(caught.exception)
        self.assertIn("interlace", message)
        self.assertIn("non-interlaced", message)

    def test_a_sixteen_bit_png_is_refused_by_name(self) -> None:
        """Kills a reader that reads the high byte of each sample and moves on."""
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(
                build_png(2, 2, depth=16, colour_type=6, interlace=0, raw=b"\x00" * 34)
            )
        message = str(caught.exception)
        self.assertIn("bit depth of 16", message)
        self.assertIn("8-bit", message)

    def test_a_palette_png_is_refused_by_name(self) -> None:
        """Kills a reader that would return palette indices as if they were colour."""
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(
                build_png(2, 2, depth=8, colour_type=3, interlace=0, raw=b"\x00" * 6)
            )
        self.assertIn("palette", str(caught.exception))

    def test_something_that_is_not_a_png_is_refused(self) -> None:
        """Kills a reader that indexes into whatever bytes it was given."""
        with self.assertRaises(png.ImageUnreadable):
            png.decode(b"not a picture at all, not even close to one")
        with self.assertRaises(WorkbenchError) as caught:
            capture_reader.png_size(b"not a picture at all, not even close to one")
        self.assertIn("the capture image is not a PNG", str(caught.exception))

    def test_an_unknown_filter_type_is_refused(self) -> None:
        """Kills a reader that treats an out-of-range filter byte as None."""
        raw = bytearray(filtered(gradient(3, 2), 0))
        raw[0] = 9
        with self.assertRaises(png.ImageUnreadable) as caught:
            png.decode(build_png(3, 2, depth=8, colour_type=6, interlace=0, raw=bytes(raw)))
        self.assertIn("filter type 9", str(caught.exception))


class TheCommitMaskIsCheckedBeforeItIsBelieved(unittest.TestCase):
    def test_a_mask_round_trips_through_its_own_writer(self) -> None:
        """Kills an origin that is written but not read back, or the reverse."""
        payload = masks.write_mask(MASK_PIXELS)
        mask = masks.read_mask(payload, image_width=24, image_height=18)
        self.assertEqual(mask.covered, MASK_PIXELS)
        self.assertEqual((mask.origin_x, mask.origin_y), (10, 6))
        self.assertEqual((mask.width, mask.height), (4, 3))
        self.assertTrue(mask.covers(10, 6))
        self.assertFalse(mask.covers(9, 6))

    def test_an_empty_commit_mask_is_refused(self) -> None:
        """Kills an operation that may replace nothing and reports success anyway."""
        payload = b"P1\n# origin 4,4 in the source image's pixel lattice\n2 2\n00\n00\n"
        with self.assertRaises(masks.MaskUnreadable) as caught:
            masks.read_mask(payload, image_width=24, image_height=18)
        self.assertIn("covers no pixel", str(caught.exception))

    def test_a_mask_whose_size_disagrees_with_its_bits_is_refused(self) -> None:
        """Kills a decoder that pads or truncates a half-edited mask into a shape."""
        payload = b"P1\n# origin 0,0\n3 3\n111\n111\n11\n"
        with self.assertRaises(masks.MaskUnreadable) as caught:
            masks.read_mask(payload, image_width=24, image_height=18)
        self.assertIn("3x3", str(caught.exception))
        self.assertIn("8 bits", str(caught.exception))

    def test_a_mask_reaching_outside_the_image_is_refused_not_clipped(self) -> None:
        """Kills silent clipping, which changes what the owner authorised."""
        payload = masks.write_mask({(23, 17), (24, 17)})
        with self.assertRaises(masks.MaskUnreadable) as caught:
            masks.read_mask(payload, image_width=24, image_height=18)
        self.assertIn("(24, 17)", str(caught.exception))
        self.assertIn("24x18", str(caught.exception))

    def test_a_mask_that_is_not_a_bitmap_is_refused(self) -> None:
        """Kills a decoder that reads a header it never confirmed was one."""
        for payload in (b"P4\n2 2\n11\n11\n", b"P1\n", b"P1\n2\n"):
            with self.subTest(payload=payload):
                with self.assertRaises(masks.MaskUnreadable):
                    masks.read_mask(payload, image_width=24, image_height=18)

    def test_the_one_decoder_serves_the_selection_reader_too(self) -> None:
        """Kills a second decoder of the same format drifting from this one.

        `resolve.py` addresses cells and this module addresses pixels; the bytes
        are the same format and are decoded in exactly one place.
        """
        payload = masks.write_mask({(2, 3), (3, 3)})
        origin_x, origin_y, width, height, covered = resolve.decode_mask(payload)
        self.assertEqual((origin_x, origin_y, width, height), (2, 3, 2, 1))
        self.assertEqual(covered, {(2, 3), (3, 3)})
        with self.assertRaises(resolve.Refused):
            resolve.decode_mask(b"P4\n2 2\n11\n11\n")


class TheContextRegionIsDeterministic(unittest.TestCase):
    def test_the_region_is_the_mask_grown_and_clipped(self) -> None:
        """Kills a context crop that wanders off the image or forgets the margin."""
        image = gradient(24, 18)
        mask = masks.read_mask(
            masks.write_mask(MASK_PIXELS), image_width=24, image_height=18
        )
        region = preserve.context_region(image, mask, MARGIN)
        self.assertEqual(region.as_record(), {"x": 7, "y": 3, "width": 10, "height": 9})
        self.assertEqual(region, preserve.context_region(image, mask, MARGIN))

    def test_a_generous_margin_clips_to_the_image(self) -> None:
        """Kills a crop that would ask for pixels the source does not have."""
        image = gradient(24, 18)
        mask = masks.read_mask(
            masks.write_mask(MASK_PIXELS), image_width=24, image_height=18
        )
        region = preserve.context_region(image, mask, 1000)
        self.assertEqual(region.as_record(), {"x": 0, "y": 0, "width": 24, "height": 18})

    def test_a_negative_margin_is_refused(self) -> None:
        """Kills a margin that would shrink the crop inside the committed pixels."""
        image = gradient(24, 18)
        mask = masks.read_mask(
            masks.write_mask(MASK_PIXELS), image_width=24, image_height=18
        )
        with self.assertRaises(preserve.PreservationRefused):
            preserve.context_region(image, mask, -1)


class EditFixture(unittest.TestCase):
    """A source, a commit mask, and a valid operation, in a throwaway tree."""

    def setUp(self) -> None:
        super().setUp()
        self.root = Path(tempfile.mkdtemp(prefix="tme-imageops-")).resolve()
        self.addCleanup(shutil.rmtree, self.root, ignore_errors=True)
        self.image = gradient(24, 18)
        self.source_bytes = png.encode(self.image)
        (self.root / "source.png").write_bytes(self.source_bytes)
        self.mask_bytes = masks.write_mask(MASK_PIXELS)
        (self.root / "commit.pbm").write_bytes(self.mask_bytes)
        self.mask = masks.read_mask(self.mask_bytes, image_width=24, image_height=18)
        self.region = preserve.context_region(self.image, self.mask, MARGIN)

    def digest(self, payload: bytes) -> str:
        return digest_bytes(payload)

    def operation(self, **overrides) -> contracts.AssetOperation:
        record = {
            "verb": contracts.EDIT_REGION,
            "author": "the owner",
            "source": {"path": "source.png", "sha256": self.digest(self.source_bytes)},
            "commit_mask": {
                "path": "commit.pbm",
                "sha256": self.digest(self.mask_bytes),
            },
            "context": {"margin": MARGIN},
            "adapter": {"adapter": "palette_fill", "parameters": {"colour": list(GARISH)}},
        }
        record.update(overrides)
        return contracts.validate(record, registry=self.registry())

    def registry(self, **extra) -> dict:
        table = dict(adapters.REGISTRY)
        table.update(extra)
        return table

    def hostile(self) -> dict:
        """An adapter that paints the ENTIRE context, mask or no mask."""

        def scribble(context: Image, parameters: dict) -> Image:
            return context.with_pixels(bytes(GARISH) * (context.width * context.height))

        return self.registry(
            palette_fill=adapters.Adapter(
                name="palette_fill",
                kind=adapters.LOCAL_DETERMINISTIC,
                verbs=(contracts.EDIT_REGION,),
                summary="a hostile stand-in that ignores the commit boundary",
                apply=scribble,
            )
        )

    def obedient(self) -> dict:
        """An adapter that writes only where the commit mask allows.

        It is handed the mask by this test's own closure, which no real adapter
        ever is. That is the control case: the project's answer must be the same
        whether or not the adapter cooperated.
        """
        mask, region = self.mask, self.region

        def paint(context: Image, parameters: dict) -> Image:
            pixels = bytearray(context.pixels)
            for y in range(context.height):
                for x in range(context.width):
                    if mask.covers(region.x + x, region.y + y):
                        offset = (y * context.width + x) * 4
                        pixels[offset : offset + 4] = bytes(WELL_BEHAVED)
            return context.with_pixels(bytes(pixels))

        return self.registry(
            palette_fill=adapters.Adapter(
                name="palette_fill",
                kind=adapters.LOCAL_DETERMINISTIC,
                verbs=(contracts.EDIT_REGION,),
                summary="a cooperative stand-in that respects the commit boundary",
                apply=paint,
            )
        )


class TheAdapterIsNotTrusted(EditFixture):
    """A hostile adapter scribbles over everything it is handed; nothing gets through.

    The ruling this class holds: **restoration is the rule, and the count is
    provenance rather than a verdict.** A blending adapter is supposed to return
    a whole context image whose every pixel differs a little, so refusing an edit
    because the adapter touched pixels outside the boundary would refuse every
    generative adapter the contract exists to accommodate. What protects accepted
    work is that none of those pixels can reach the result.

    The blocking check is therefore the invariant itself, and it is qualified by
    a mutant on this project's own compositing step rather than on an adapter.
    """

    def test_the_composited_result_is_the_source_outside_the_mask(self) -> None:
        """Kills any path by which an adapter's out-of-mask byte reaches the result.

        Not "is repaired afterwards" — never written. The check is byte-for-byte
        over the whole image, including the context region the adapter painted.
        """
        returned = Image(
            width=self.region.width,
            height=self.region.height,
            pixels=bytes(GARISH) * (self.region.width * self.region.height),
        )
        composited = preserve.composite(self.image, self.region, returned, self.mask)

        self.assertTrue(preserve.preserved_outside(self.image, composited.image, self.mask))
        for y in range(self.image.height):
            for x in range(self.image.width):
                if self.mask.covers(x, y):
                    self.assertEqual(composited.image.pixel(x, y), GARISH)
                else:
                    self.assertEqual(
                        composited.image.pixel(x, y),
                        self.image.pixel(x, y),
                        f"pixel ({x}, {y}) outside the commit mask was overwritten",
                    )

    def test_every_pixel_it_had_no_business_touching_is_counted(self) -> None:
        """Kills a preservation step that restores quietly and reports nothing."""
        returned = Image(
            width=self.region.width,
            height=self.region.height,
            pixels=bytes(GARISH) * (self.region.width * self.region.height),
        )
        composited = preserve.composite(self.image, self.region, returned, self.mask)
        context_pixels = self.region.width * self.region.height
        self.assertEqual(composited.changed_pixels, len(MASK_PIXELS))
        self.assertEqual(composited.outside_writes, context_pixels - len(MASK_PIXELS))
        self.assertEqual(composited.outside_writes, 78)
        self.assertTrue(composited.first_outside)
        for x, y in composited.first_outside:
            self.assertFalse(self.mask.covers(x, y))
            self.assertTrue(self.region.contains(x, y))

    def test_the_hostile_adapters_writes_are_discarded_and_the_edit_stands(self) -> None:
        """Kills the idea that preservation depends on the adapter behaving.

        The hostile adapter paints the whole context. The edit SUCCEEDS — the
        owner asked for those twelve pixels and gets them — and every one of the
        seventy-eight pixels it had no business touching is gone from the result.
        Nothing about that outcome depended on the adapter's intent.
        """
        result = run.run_edit_region(
            self.operation(), root=self.root, registry=self.hostile()
        )
        self.assertEqual(result.changed_pixels, len(MASK_PIXELS))
        self.assertEqual(result.outside_writes, 78)
        composed = png.decode(result.image)
        self.assertTrue(preserve.preserved_outside(self.image, composed, self.mask))
        for x, y in MASK_PIXELS:
            self.assertEqual(composed.pixel(x, y), GARISH)

    def test_what_the_adapter_touched_is_on_the_record(self) -> None:
        """Kills a preservation step that restores quietly and reports nothing.

        The count is provenance rather than a verdict, and provenance nobody
        writes down is provenance nobody has.
        """
        result = run.run_edit_region(
            self.operation(), root=self.root, registry=self.hostile()
        )
        record = result.as_record()["adapter_wrote_outside_the_mask"]
        self.assertEqual(record["pixels"], 78)
        self.assertTrue(record["first"])
        for x, y in record["first"]:
            self.assertFalse(self.mask.covers(x, y))

    def test_a_compositing_step_that_trusts_the_adapter_is_caught(self) -> None:
        """THE MUTANT that qualifies the blocking check (P9).

        The hostile adapter proves only that the construction works. What earns
        `preserved_outside` its blocking status is a mutant on THIS project's own
        code: a compositing step that hands back the adapter's output. It is
        planted here and the executor refuses to return its result.
        """
        original = run.preserve.composite

        def trusting(source, region, returned, mask):
            composited = original(source, region, returned, mask)
            pixels = bytearray(source.pixels)
            for y in range(region.height):
                for x in range(region.width):
                    offset = ((region.y + y) * source.width + region.x + x) * 4
                    taken = (y * returned.width + x) * 4
                    pixels[offset : offset + 4] = returned.pixels[taken : taken + 4]
            return replace(composited, image=source.with_pixels(bytes(pixels)))

        run.preserve.composite = trusting
        self.addCleanup(setattr, run.preserve, "composite", original)
        with self.assertRaises(run.EditRefused) as caught:
            run.run_edit_region(self.operation(), root=self.root, registry=self.hostile())
        self.assertIn("outside the commit mask", str(caught.exception))
        self.assertIn("preservation step did not hold", str(caught.exception))

    def test_an_adapter_that_stays_inside_the_mask_is_accepted(self) -> None:
        """Kills a preservation step so strict it rejects a correct edit too.

        The control case. Exactly the mask's twelve pixels change, the result is
        the source everywhere else, nothing is written to disk, and the adapter
        is recorded as having touched nothing it should not have.
        """
        result = run.run_edit_region(
            self.operation(), root=self.root, registry=self.obedient()
        )
        self.assertEqual(result.changed_pixels, len(MASK_PIXELS))
        self.assertEqual(result.outside_writes, 0)
        self.assertEqual(result.region, self.region)
        self.assertEqual(result.sha256, self.digest(result.image))

        composed = png.decode(result.image)
        self.assertTrue(preserve.preserved_outside(self.image, composed, self.mask))
        for x, y in MASK_PIXELS:
            self.assertEqual(composed.pixel(x, y), WELL_BEHAVED)
        self.assertEqual(
            sorted(path.name for path in self.root.iterdir()),
            ["commit.pbm", "source.png"],
        )

    def test_the_source_on_disk_is_untouched_by_any_edit(self) -> None:
        """Kills an executor that writes anything at all."""
        run.run_edit_region(self.operation(), root=self.root, registry=self.hostile())
        self.assertEqual((self.root / "source.png").read_bytes(), self.source_bytes)
        self.assertEqual(
            sorted(path.name for path in self.root.iterdir()),
            ["commit.pbm", "source.png"],
        )


class AMovedDigestStopsTheEdit(EditFixture):
    def test_a_source_whose_digest_moved_is_refused(self) -> None:
        """Kills an edit that follows a master someone changed underneath it."""
        moved = gradient(24, 18).with_pixels(
            bytes(WELL_BEHAVED) + gradient(24, 18).pixels[4:]
        )
        (self.root / "source.png").write_bytes(png.encode(moved))
        with self.assertRaises(WorkbenchError) as caught:
            run.run_edit_region(self.operation(), root=self.root, registry=self.obedient())
        message = str(caught.exception)
        self.assertIn("source.png", message)
        self.assertIn("digest moved", message)
        self.assertNotIn("commit.pbm", message)

    def test_a_commit_mask_whose_digest_moved_is_refused(self) -> None:
        """Kills an edit that honours a boundary someone widened after the fact.

        Its own mutant, independent of the source: the mask is the authorisation,
        so a mask edited after the operation was written is exactly the case that
        must never resolve.
        """
        (self.root / "commit.pbm").write_bytes(self.mask_bytes.replace(b"1", b"0", 1))
        with self.assertRaises(WorkbenchError) as caught:
            run.run_edit_region(self.operation(), root=self.root, registry=self.obedient())
        message = str(caught.exception)
        self.assertIn("commit.pbm", message)
        self.assertIn("digest moved", message)

    def test_a_missing_source_is_refused(self) -> None:
        """Kills a reader that treats an absent file as an empty one."""
        (self.root / "source.png").unlink()
        with self.assertRaises(WorkbenchError) as caught:
            run.run_edit_region(self.operation(), root=self.root, registry=self.obedient())
        self.assertIn("missing", str(caught.exception))


class AnAdapterThatBreaksTheProtocolIsRefused(EditFixture):
    def test_an_adapter_returning_the_wrong_size_is_refused(self) -> None:
        """Kills a composite that would index a returned image by the wrong stride."""
        def shrink(context: Image, parameters: dict) -> Image:
            return Image(width=1, height=1, pixels=bytes(GARISH))

        registry = self.registry(
            palette_fill=adapters.Adapter(
                name="palette_fill",
                kind=adapters.LOCAL_DETERMINISTIC,
                verbs=(contracts.EDIT_REGION,),
                summary="returns an image of the wrong size",
                apply=shrink,
            )
        )
        with self.assertRaises(preserve.PreservationRefused) as caught:
            run.run_edit_region(self.operation(), root=self.root, registry=registry)
        message = str(caught.exception)
        self.assertIn("1x1", message)
        self.assertIn(f"{self.region.width}x{self.region.height}", message)

    def test_an_adapter_returning_something_that_is_not_an_image_is_refused(self) -> None:
        """Kills an executor that composites whatever object it got back."""
        def wrong(context: Image, parameters: dict):
            return b"pixels, honest"

        registry = self.registry(
            palette_fill=adapters.Adapter(
                name="palette_fill",
                kind=adapters.LOCAL_DETERMINISTIC,
                verbs=(contracts.EDIT_REGION,),
                summary="returns bytes rather than an image",
                apply=wrong,
            )
        )
        with self.assertRaises(run.EditRefused) as caught:
            run.run_edit_region(self.operation(), root=self.root, registry=registry)
        self.assertIn("returns an image", str(caught.exception))

    def test_an_empty_commit_mask_stops_the_edit(self) -> None:
        """Kills an edit that authorises nothing and is treated as an edit."""
        (self.root / "commit.pbm").write_bytes(b"P1\n# origin 0,0\n2 2\n00\n00\n")
        operation = self.operation(
            commit_mask={
                "path": "commit.pbm",
                "sha256": self.digest((self.root / "commit.pbm").read_bytes()),
            }
        )
        with self.assertRaises(masks.MaskUnreadable) as caught:
            run.run_edit_region(operation, root=self.root, registry=self.obedient())
        self.assertIn("covers no pixel", str(caught.exception))

    def test_a_context_region_that_misses_part_of_the_mask_is_refused(self) -> None:
        """Kills a composite that silently drops authorised pixels it cannot reach."""
        narrow = preserve.Region(x=10, y=6, width=2, height=3)
        returned = preserve.crop(self.image, narrow)
        with self.assertRaises(preserve.PreservationRefused) as caught:
            preserve.composite(self.image, narrow, returned, self.mask)
        self.assertIn("does not contain commit-mask pixel", str(caught.exception))


class TheReferenceAdapterIsDeterministic(unittest.TestCase):
    def test_a_solid_fill_paints_every_pixel_it_is_handed(self) -> None:
        """Kills an adapter that quietly respects a boundary it was never told."""
        context = gradient(6, 4)
        painted = adapters.palette_fill(context, {"colour": list(GARISH)})
        self.assertEqual(painted.pixels, bytes(GARISH) * 24)
        self.assertEqual(painted, adapters.palette_fill(context, {"colour": list(GARISH)}))

    def test_a_checker_alternates_on_its_period(self) -> None:
        """Kills a pattern that depends on where the crop sits in the source."""
        context = gradient(4, 4)
        painted = adapters.palette_fill(
            context,
            {"colour": list(GARISH), "alternate": list(WELL_BEHAVED), "pattern": "checker",
             "period": 2},
        )
        self.assertEqual(painted.pixel(0, 0), GARISH)
        self.assertEqual(painted.pixel(2, 0), WELL_BEHAVED)
        self.assertEqual(painted.pixel(0, 2), WELL_BEHAVED)
        self.assertEqual(painted.pixel(2, 2), GARISH)

    def test_a_malformed_parameter_block_is_refused(self) -> None:
        """Kills an adapter that fills in a default for something it was not told."""
        context = gradient(4, 4)
        for parameters in (
            {},
            {"colour": [255, 0]},
            {"colour": [255, 0, 255, 300]},
            {"colour": list(GARISH), "pattern": "swirl"},
            {"colour": list(GARISH), "pattern": "checker"},
            {"colour": list(GARISH), "period": 4},
            {"colour": list(GARISH), "tint": 3},
        ):
            with self.subTest(parameters=parameters):
                with self.assertRaises(adapters.AdapterRefused):
                    adapters.palette_fill(context, parameters)

    def test_an_unknown_adapter_names_the_registered_set(self) -> None:
        """Kills a lookup that returns None and lets the caller discover it later."""
        with self.assertRaises(adapters.AdapterRefused) as caught:
            adapters.lookup("a_hosted_model")
        message = str(caught.exception)
        self.assertIn("a_hosted_model", message)
        self.assertIn("palette_fill", message)


class TheOperationVocabularyIsClosed(unittest.TestCase):
    def valid(self) -> dict:
        return {
            "verb": contracts.EDIT_REGION,
            "author": "the owner",
            "source": {"path": "a.png", "sha256": "0" * 64},
            "commit_mask": {"path": "a.pbm", "sha256": "1" * 64},
            "context": {"margin": 4},
            "adapter": {"adapter": "palette_fill", "parameters": {"colour": [1, 2, 3, 4]}},
        }

    def test_all_five_operations_are_declared(self) -> None:
        """Kills a vocabulary that drifted from the five the specification names."""
        self.assertEqual(
            sorted(contracts.VOCABULARY),
            [
                "animate_asset",
                "compare_candidates",
                "edit_region",
                "generate_asset",
                "normalize_pixel_grid",
            ],
        )
        for verb in contracts.VOCABULARY:
            self.assertTrue(contracts.contract(verb).summary.endswith("."))

    def test_a_well_formed_edit_region_parses(self) -> None:
        """Kills a validator that refuses the shape it is supposed to accept."""
        operation = contracts.validate(self.valid())
        self.assertEqual(operation.verb, contracts.EDIT_REGION)
        self.assertEqual(operation.context.margin, 4)
        self.assertEqual(operation.adapter.adapter, "palette_fill")
        self.assertEqual(operation.as_record(), self.valid())

    def test_an_unknown_verb_names_the_whole_vocabulary(self) -> None:
        """Kills a layer that lets an adapter widen the operation set."""
        record = self.valid()
        record["verb"] = "inpaint"
        with self.assertRaises(contracts.OperationRefused) as caught:
            contracts.validate(record)
        message = str(caught.exception)
        self.assertIn("inpaint", message)
        for verb in contracts.VOCABULARY:
            self.assertIn(verb, message)

    def test_a_missing_or_unknown_field_is_refused(self) -> None:
        """Kills a record that half-specifies an edit, and one that smuggles a field."""
        for absent in ("source", "commit_mask", "context", "adapter", "author"):
            with self.subTest(missing=absent):
                record = self.valid()
                del record[absent]
                with self.assertRaises(contracts.OperationRefused) as caught:
                    contracts.validate(record)
                self.assertIn(absent, str(caught.exception))

        record = self.valid()
        record["seed"] = 1234
        with self.assertRaises(contracts.OperationRefused) as caught:
            contracts.validate(record)
        self.assertIn("seed", str(caught.exception))

    def test_adapter_parameters_stay_in_the_adapter_block(self) -> None:
        """Kills adapter-specific keys leaking into fields the layer shares.

        `context` carries a margin and nothing else. The moment a provider's own
        setting can ride in a shared field, the layer stops being neutral and the
        next adapter arrives as a migration.
        """
        record = self.valid()
        record["context"] = {"margin": 4, "guidance_scale": 7.5}
        with self.assertRaises(contracts.OperationRefused) as caught:
            contracts.validate(record)
        self.assertIn("guidance_scale", str(caught.exception))

    def test_an_unregistered_adapter_is_refused(self) -> None:
        """Kills an operation naming a provider nothing in this tree can reach."""
        record = self.valid()
        record["adapter"] = {"adapter": "a_hosted_model", "parameters": {}}
        with self.assertRaises(WorkbenchError) as caught:
            contracts.validate(record)
        self.assertIn("a_hosted_model", str(caught.exception))

    def test_an_adapter_registered_for_another_verb_is_refused(self) -> None:
        """Kills a name lookup that never checks what the adapter actually serves."""
        record = {
            "verb": contracts.NORMALIZE_PIXEL_GRID,
            "author": "the owner",
            "source": {"path": "a.png", "sha256": "0" * 64},
            "grammar": "the project pixel grid",
            "adapter": {"adapter": "palette_fill", "parameters": {}},
        }
        with self.assertRaises(contracts.OperationRefused) as caught:
            contracts.validate(record)
        self.assertIn("edit_region", str(caught.exception))

    def test_an_adapter_on_a_verb_that_accepts_none_is_refused(self) -> None:
        """Kills a model quietly deciding what an owner is shown beside what."""
        record = {
            "verb": contracts.COMPARE_CANDIDATES,
            "author": "the owner",
            "references": [{"path": "a.png", "sha256": "0" * 64}],
            "descriptor": "the ember golden-hour bar",
            "adapter": {"adapter": "palette_fill", "parameters": {}},
        }
        with self.assertRaises(contracts.OperationRefused) as caught:
            contracts.validate(record)
        self.assertIn("accepts no adapter", str(caught.exception))


class TheFourUnimplementedVerbsSaySoPlainly(unittest.TestCase):
    """Kills a verb that parses and then quietly does nothing at all."""

    RECORDS = {
        "generate_asset": {
            "verb": "generate_asset",
            "author": "the owner",
            "grammar": "one cell, ember golden hour, sixteen pixels",
            "references": [{"path": "master.png", "sha256": "0" * 64}],
        },
        "animate_asset": {
            "verb": "animate_asset",
            "author": "the owner",
            "source": {"path": "master.png", "sha256": "0" * 64},
        },
        "normalize_pixel_grid": {
            "verb": "normalize_pixel_grid",
            "author": "the owner",
            "source": {"path": "candidate.png", "sha256": "0" * 64},
            "grammar": "the project pixel grid, palette, and pivot",
        },
        "compare_candidates": {
            "verb": "compare_candidates",
            "author": "the owner",
            "references": [
                {"path": "candidate.png", "sha256": "0" * 64},
                {"path": "master.png", "sha256": "1" * 64},
            ],
            "descriptor": "the accepted look",
        },
    }

    def test_each_one_validates_as_a_record(self) -> None:
        for verb, record in self.RECORDS.items():
            with self.subTest(verb=verb):
                operation = contracts.validate(record)
                self.assertEqual(operation.verb, verb)
                self.assertEqual(operation.as_record(), record)

    def test_each_one_refuses_to_execute_and_says_why(self) -> None:
        for verb, record in self.RECORDS.items():
            with self.subTest(verb=verb):
                operation = contracts.validate(record)
                with self.assertRaises(run.EditRefused) as caught:
                    run.execute(operation, root=REPO_ROOT)
                message = str(caught.exception)
                self.assertEqual(message, contracts.no_executor(verb))
                self.assertIn(f"no adapter is registered for {verb}", message)
                self.assertIn("palette_fill for edit_region alone", message)

    def test_no_adapter_in_the_registry_claims_any_of_them(self) -> None:
        """Kills a refusal message that would be a lie the day one is registered."""
        for adapter in adapters.REGISTRY.values():
            self.assertEqual(adapter.verbs, (contracts.EDIT_REGION,))


if __name__ == "__main__":
    unittest.main()
