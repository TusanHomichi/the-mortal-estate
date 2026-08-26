"""A PNG reader and writer, written here because the Workbench has no imaging library.

The package takes no dependency outside the standard library, so the choice is
between a real decoder and a tool that can only ever look at a picture's header.
`capture.py` lived with the second option for as long as reading the header was
all anyone needed. An operation layer that composites pixels needs the first,
and one parser is the whole point: `capture.png_size` now delegates here rather
than keeping a second, weaker reader alive beside this one.

**The subset is 8-bit and non-interlaced, and it is a bound limit, not a stage.**
Colour types 0, 2, 4, and 6 decode; 16-bit samples, palettes, and Adam7
interlacing are refused by name. The refusal is the honest answer because the
alternative — quietly averaging a 16-bit sample down to eight, or picking a
palette entry — produces pixels the project would then treat as accepted work.
A picture this reader cannot read is a picture the project does not composite.

**Every chunk's CRC is checked.** A PNG carries a checksum per chunk; ignoring
it means a corrupted master decodes into plausible garbage and gets committed.
Reading the four bytes costs nothing and turns silent corruption into a refusal.

**Encoding is deterministic.** Filter type 0 on every row, one IDAT, one fixed
compression level, no ancillary chunks: the same `Image` encodes to the same
bytes on every machine and every run. Downstream, a candidate's identity is its
SHA-256, and a digest that moves because the encoder felt different today is a
digest that proves nothing.
"""

from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass

from ..projection import WorkbenchError

SIGNATURE = b"\x89PNG\r\n\x1a\n"

IHDR = b"IHDR"
IDAT = b"IDAT"
IEND = b"IEND"
TRNS = b"tRNS"

#: Signature (8) + length (4) + type (4) + IHDR data (13) + CRC (4).
MINIMUM_HEADER_BYTES = 33

#: The colour types this reader handles, and how many samples each pixel carries.
CHANNELS = {0: 1, 2: 3, 4: 2, 6: 4}

COLOUR_NAMES = {
    0: "greyscale",
    2: "truecolour",
    3: "palette",
    4: "greyscale with alpha",
    6: "truecolour with alpha",
}

#: Fixed so that two encodes of one image are byte-identical. Level 9 because
#: these are stored artifacts, not a hot path.
COMPRESSION_LEVEL = 9

#: RGBA8: four bytes per pixel, and the only shape `Image` holds.
BYTES_PER_PIXEL = 4


class ImageUnreadable(WorkbenchError):
    """A payload is not a picture this reader handles, and the reason names why.

    Refusal rather than a best-effort decode. Every consumer of this module
    goes on to composite the result over accepted work, so "probably these
    pixels" is the one answer that must never be returned.
    """


@dataclass(frozen=True)
class Image:
    """One decoded picture: RGBA8, row-major, no stride and no padding.

    The invariant is checked on construction rather than trusted, because every
    other module in this package indexes `pixels` arithmetically. A buffer that
    is one row short would otherwise read a neighbour's bytes and composite
    them.
    """

    width: int
    height: int
    pixels: bytes

    def __post_init__(self) -> None:
        if self.width <= 0 or self.height <= 0:
            raise ImageUnreadable(
                f"an image of {self.width}x{self.height} has no pixels to hold"
            )
        expected = self.width * self.height * BYTES_PER_PIXEL
        if len(self.pixels) != expected:
            raise ImageUnreadable(
                f"a {self.width}x{self.height} RGBA image needs {expected} bytes "
                f"and this one carries {len(self.pixels)}"
            )

    def offset(self, x: int, y: int) -> int:
        """The byte offset of one pixel, refusing a coordinate outside the picture."""
        if x < 0 or y < 0 or x >= self.width or y >= self.height:
            raise ImageUnreadable(
                f"pixel ({x}, {y}) is outside a {self.width}x{self.height} image"
            )
        return (y * self.width + x) * BYTES_PER_PIXEL

    def pixel(self, x: int, y: int) -> tuple[int, int, int, int]:
        start = self.offset(x, y)
        red, green, blue, alpha = self.pixels[start : start + BYTES_PER_PIXEL]
        return int(red), int(green), int(blue), int(alpha)

    def with_pixels(self, pixels: bytes) -> "Image":
        """The same frame carrying different pixels, validated the same way."""
        return Image(width=self.width, height=self.height, pixels=bytes(pixels))


def _crc(chunk_type: bytes, data: bytes) -> int:
    return zlib.crc32(chunk_type + data) & 0xFFFFFFFF


def _read_chunk(payload: bytes, offset: int) -> tuple[bytes, bytes, int]:
    """One chunk and the offset after it, with its CRC checked."""
    if offset + 8 > len(payload):
        raise ImageUnreadable(
            f"the PNG ends after {len(payload)} bytes, inside a chunk header"
        )
    length, chunk_type = struct.unpack(">I4s", payload[offset : offset + 8])
    end = offset + 8 + length + 4
    if end > len(payload):
        raise ImageUnreadable(
            f"the PNG declares a {length}-byte {chunk_type.decode('ascii', 'replace')} "
            f"chunk at offset {offset}, and the payload ends {end - len(payload)} bytes early"
        )
    data = payload[offset + 8 : offset + 8 + length]
    declared = struct.unpack(">I", payload[offset + 8 + length : end])[0]
    actual = _crc(chunk_type, data)
    if declared != actual:
        raise ImageUnreadable(
            f"the {chunk_type.decode('ascii', 'replace')} chunk at offset {offset} "
            f"declares CRC {declared:08x} and its bytes hash to {actual:08x}"
        )
    return chunk_type, data, end


def _header(payload: bytes) -> tuple[int, int, int, int, int, int, int]:
    """The IHDR fields, after the signature and the header CRC have been checked."""
    if not payload.startswith(SIGNATURE):
        raise ImageUnreadable("the payload does not start with the PNG signature")
    if len(payload) < MINIMUM_HEADER_BYTES:
        raise ImageUnreadable(
            f"the payload is {len(payload)} bytes and a PNG header alone needs "
            f"{MINIMUM_HEADER_BYTES}"
        )
    chunk_type, data, _ = _read_chunk(payload, 8)
    if chunk_type != IHDR or len(data) != 13:
        raise ImageUnreadable(
            f"the first chunk is {chunk_type.decode('ascii', 'replace')!r} of "
            f"{len(data)} bytes, and a PNG opens with a 13-byte IHDR"
        )
    width, height, depth, colour, compression, filter_method, interlace = struct.unpack(
        ">IIBBBBB", data
    )
    if width == 0 or height == 0:
        raise ImageUnreadable(f"the PNG declares {width}x{height}, which has no pixels")
    return int(width), int(height), depth, colour, filter_method, interlace, compression


def size(payload: bytes) -> tuple[int, int]:
    """The width and height a PNG declares.

    Separate from `decode` because a consumer that only needs to know whether a
    sidecar's viewport matches its picture should not pay to inflate the whole
    image. The signature and the header's own CRC are still checked, so this is
    a cheap answer rather than a credulous one.
    """
    width, height, *_ = _header(payload)
    return width, height


def decode(payload: bytes) -> Image:
    """Decode an 8-bit non-interlaced PNG into RGBA8, or refuse naming what it is."""
    width, height, depth, colour, filter_method, interlace, compression = _header(payload)
    if interlace != 0:
        raise ImageUnreadable(
            f"the PNG declares interlace method {interlace} (Adam7); this reader "
            "handles 8-bit non-interlaced PNG only"
        )
    if depth != 8:
        raise ImageUnreadable(
            f"the PNG declares a bit depth of {depth}; this reader handles 8-bit "
            "non-interlaced PNG only"
        )
    if colour not in CHANNELS:
        raise ImageUnreadable(
            f"the PNG declares colour type {colour} "
            f"({COLOUR_NAMES.get(colour, 'unknown')}); this reader handles 8-bit "
            "non-interlaced greyscale, greyscale with alpha, truecolour, and "
            "truecolour with alpha"
        )
    if compression != 0 or filter_method != 0:
        raise ImageUnreadable(
            f"the PNG declares compression method {compression} and filter method "
            f"{filter_method}; PNG defines only 0 for each"
        )

    channels = CHANNELS[colour]
    compressed = bytearray()
    seen_end = False
    offset = 8 + 8 + 13 + 4  # past the signature and the IHDR chunk
    while offset < len(payload) and not seen_end:
        chunk_type, data, offset = _read_chunk(payload, offset)
        if chunk_type == IDAT:
            # Concatenated before inflating: PNG allows an encoder to split one
            # zlib stream across any number of IDAT chunks at any byte, and the
            # tracked capture does exactly that.
            compressed += data
        elif chunk_type == TRNS:
            raise ImageUnreadable(
                "the PNG carries a tRNS chunk; this reader does not apply declared "
                "transparency and will not return pixels that ignore it"
            )
        elif chunk_type == IEND:
            seen_end = True
    if not seen_end:
        raise ImageUnreadable("the PNG carries no IEND chunk; it is truncated")
    if not compressed:
        raise ImageUnreadable("the PNG carries no IDAT chunk; it has no pixels")

    try:
        raw = zlib.decompress(bytes(compressed))
    except zlib.error as error:
        raise ImageUnreadable(f"the PNG's pixel stream does not inflate: {error}") from error

    stride = width * channels
    if len(raw) != height * (stride + 1):
        raise ImageUnreadable(
            f"the PNG inflates to {len(raw)} bytes and a {width}x{height} image with "
            f"{channels} channels needs {height * (stride + 1)}"
        )
    samples = _unfilter(raw, width, height, channels)
    return Image(width=width, height=height, pixels=_to_rgba(samples, colour, channels))


def _paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    to_left = abs(estimate - left)
    to_above = abs(estimate - above)
    to_corner = abs(estimate - upper_left)
    if to_left <= to_above and to_left <= to_corner:
        return left
    if to_above <= to_corner:
        return above
    return upper_left


def _unfilter(raw: bytes, width: int, height: int, channels: int) -> bytearray:
    """Reverse the per-row filter, all five types of it.

    Every row names its own filter, so a reader that implements four of the five
    decodes most pictures and silently mangles the rest. There is no partial
    credit here: the five are implemented, and an unknown type is refused.
    """
    stride = width * channels
    out = bytearray(height * stride)
    previous = bytearray(stride)
    source = 0
    for row in range(height):
        kind = raw[source]
        source += 1
        line = bytearray(raw[source : source + stride])
        source += stride
        if kind == 0:
            pass
        elif kind == 1:  # Sub: the pixel to the left
            for index in range(channels, stride):
                line[index] = (line[index] + line[index - channels]) & 0xFF
        elif kind == 2:  # Up: the pixel above
            for index in range(stride):
                line[index] = (line[index] + previous[index]) & 0xFF
        elif kind == 3:  # Average: the floor of the mean of left and above
            for index in range(stride):
                left = line[index - channels] if index >= channels else 0
                line[index] = (line[index] + ((left + previous[index]) >> 1)) & 0xFF
        elif kind == 4:  # Paeth: whichever neighbour the predictor picks
            for index in range(stride):
                left = line[index - channels] if index >= channels else 0
                upper_left = previous[index - channels] if index >= channels else 0
                line[index] = (line[index] + _paeth(left, previous[index], upper_left)) & 0xFF
        else:
            raise ImageUnreadable(
                f"row {row} declares filter type {kind}; PNG defines 0 through 4"
            )
        out[row * stride : (row + 1) * stride] = line
        previous = line
    return out


def _to_rgba(samples: bytearray, colour: int, channels: int) -> bytes:
    """Widen whatever the file carried into the one shape `Image` holds.

    Alpha is filled to 255 where the file declares none, and grey is copied into
    all three colour channels. Both are exact, not approximations: an opaque
    picture is opaque, and a grey sample is that grey.
    """
    if colour == 6:
        return bytes(samples)
    count = len(samples) // channels
    out = bytearray(count * BYTES_PER_PIXEL)
    if colour == 2:
        for index in range(count):
            source = index * 3
            target = index * BYTES_PER_PIXEL
            out[target : target + 3] = samples[source : source + 3]
            out[target + 3] = 255
        return bytes(out)
    if colour == 0:
        for index in range(count):
            grey = samples[index]
            target = index * BYTES_PER_PIXEL
            out[target] = out[target + 1] = out[target + 2] = grey
            out[target + 3] = 255
        return bytes(out)
    for index in range(count):  # colour type 4: greyscale with alpha
        grey = samples[index * 2]
        target = index * BYTES_PER_PIXEL
        out[target] = out[target + 1] = out[target + 2] = grey
        out[target + 3] = samples[index * 2 + 1]
    return bytes(out)


def _chunk(chunk_type: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + chunk_type + data + struct.pack(">I", _crc(chunk_type, data))


def encode(image: Image) -> bytes:
    """One image, as an RGBA8 non-interlaced PNG, byte-identical on every call.

    No ancillary chunks are written. A colour-space or timestamp chunk would be
    a second fact about the picture that this package does not own, and a
    timestamp would make the digest of an unchanged image move.
    """
    header = struct.pack(">IIBBBBB", image.width, image.height, 8, 6, 0, 0, 0)
    stride = image.width * BYTES_PER_PIXEL
    raw = bytearray()
    for row in range(image.height):
        raw.append(0)  # filter type None on every row: nothing to reverse, nothing to guess
        raw += image.pixels[row * stride : (row + 1) * stride]
    return b"".join((
        SIGNATURE,
        _chunk(IHDR, header),
        _chunk(IDAT, zlib.compress(bytes(raw), COMPRESSION_LEVEL)),
        _chunk(IEND, b""),
    ))
