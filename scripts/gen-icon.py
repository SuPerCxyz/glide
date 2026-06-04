#!/usr/bin/env python3
"""Generate a minimal RGBA PNG icon for Glide."""
import struct, zlib, sys, os

outpath = sys.argv[1] if len(sys.argv) > 1 else "/tmp/glide.png"
size = int(sys.argv[2]) if len(sys.argv) > 2 else 256
os.makedirs(os.path.dirname(outpath) if os.path.dirname(outpath) else '.', exist_ok=True)

sig = b'\x89PNG\r\n\x1a\n'
# color_type=6 = RGBA
ihdr = struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)
ihdr_crc = struct.pack('>I', zlib.crc32(b'IHDR' + ihdr) & 0xffffffff)

raw = b''
for y in range(size):
    raw += b'\x00'  # filter byte
    for x in range(size):
        # Blue gradient with alpha=255
        raw += bytes([0x00, 0x80, 0xff, 0xff])
compressed = zlib.compress(raw)
idat_crc = struct.pack('>I', zlib.crc32(b'IDAT' + compressed) & 0xffffffff)
iend_crc = struct.pack('>I', zlib.crc32(b'IEND') & 0xffffffff)

png = (sig + struct.pack('>I', 13) + b'IHDR' + ihdr + ihdr_crc +
       struct.pack('>I', len(compressed)) + b'IDAT' + compressed + idat_crc +
       struct.pack('>I', 0) + b'IEND' + iend_crc)
with open(outpath, 'wb') as f:
    f.write(png)
print(f"Icon written to {outpath} ({len(png)} bytes, {size}x{size} RGBA)")
