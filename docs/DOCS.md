# Portable Network Graphics (PNG)
## How To Build Your Own Decoder in Rust

---

## 1. Reading The File

Read the entire file into a byte buffer, then pass it to the decoder. The decoder validates the signature and sets the starting index to `8` - right after the signature.

```rust
let data = fs::read("image.png").unwrap();
let mut decoder = PngDecoder::new(data);

// variables to fill as we parse chunks
let mut compressed_data: Vec<u8> = Vec::new();
let mut width:  usize = 0;
let mut height: usize = 0;
```

```rust
impl PngDecoder {
    fn new(data: Vec<u8>) -> Self {
        if !Self::is_png(&data) {
            panic!("Not a PNG")
        }
        Self { data, idx: 8 } // idx starts at 8 to skip the signature
    }
}
```

---

## 2. File Signature

Every PNG file starts with the same 8 magic bytes. This is the first thing you check - if it doesn't match, the file is not a valid PNG.

```rust
fn is_png(data: &[u8]) -> bool {
    data[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
}
```

---

## 2. Chunk Layout

After the 8-byte signature, the file is a sequence of **chunks**. Every chunk has exactly the same structure:

```
┌─────────────────────┬──────────────────┬───────────────────────────┬─────────┐
│   Length (4 bytes)  │  Type (4 bytes)  │   Data (Length bytes)     │  CRC    │
│   big-endian u32    │  ASCII letters   │   chunk-specific payload  │ 4 bytes │
└─────────────────────┴──────────────────┴───────────────────────────┴─────────┘
```

| Field  | Size     | Description |
|--------|----------|-------------|
| Length | 4 bytes  | Number of bytes in the Data field. Does **not** include Type or CRC. |
| Type   | 4 bytes  | ASCII name, e.g. `IHDR`, `IDAT`, `IEND`. |
| Data   | variable | Chunk payload. Can be zero bytes. |
| CRC    | 4 bytes  | CRC-32 over **Type + Data** (not Length). |

### Critical vs. Ancillary Chunks

| Category  | First Letter | Examples | Meaning |
|-----------|--------------|----------|---------|
| Critical  | Uppercase | `IHDR`, `IDAT`, `IEND` | Must be understood to decode the image |
| Ancillary | Lowercase | `tEXt`, `gAMA`, `tIME` | Safe to ignore if unrecognised |

For our decoder we only process `IHDR`, `IDAT`, and `IEND`.

---

## 3. Parsing All Chunks

Chunks start at byte index `8`, right after the signature. We loop until we hit `IEND`.

```rust
fn get_chunk(&mut self) -> Chunk {
    // --- Length (4 bytes, big-endian) ---
    let length = u32::from_be_bytes(self.move_to(4).try_into().unwrap());

    // --- Type (4 bytes, ASCII) ---
    let typ = ChunkType::from_str(&String::from_utf8(self.move_to(4)).unwrap());

    // --- Data (length bytes) ---
    let data = self.move_to(length as usize);

    // --- CRC (4 bytes) ---
    let crc = self.move_to(4);

    Chunk { length, typ, data, crc }
}
```

> **Rules you must follow:**
> - The **first** chunk must always be `IHDR`.
> - The **last** chunk must always be `IEND`.
> - There can be **multiple** `IDAT` chunks - all their data must be concatenated before decompression.

---

## 4. IHDR - Image Header

`IHDR` is always exactly **13 bytes** and contains everything you need to know about the image format.

```
Bytes  0-3  : Width               (u32, big-endian)
Bytes  4-7  : Height              (u32, big-endian)
Byte   8    : Bit depth           (u8)
Byte   9    : Color type          (u8)
Byte  10    : Compression method  (u8) - always 0
Byte  11    : Filter method       (u8) - always 0
Byte  12    : Interlace method    (u8) - 0 = none
```

### Color Types

| Value | Name            | Channels | Allowed Bit Depths |
|-------|-----------------|----------|--------------------|
| 0     | Grayscale       | 1        | 1, 2, 4, 8, 16     |
| 2     | RGB             | 3        | 8, 16              |
| 3     | Indexed         | 1        | 1, 2, 4, 8         |
| 4     | Grayscale+Alpha | 2        | 8, 16              |
| 6     | RGBA            | 4        | 8, 16              |

```rust
impl IHDR {
    fn new(data: Vec<u8>) -> Self {
        Self {
            width:              u32::from_be_bytes(data[0..4].try_into().unwrap()),
            height:             u32::from_be_bytes(data[4..8].try_into().unwrap()),
            bit_depth:          data[8],
            color_type:         data[9],
            compression_method: data[10],
            filter_method:      data[11],
            interlace_method:   data[12],
        }
    }
}
```

---

## 5. IDAT - Image Data

The pixel data is compressed with **zlib (DEFLATE)** and may be split across multiple `IDAT` chunks. You must collect them all into one buffer first, then decompress in one shot.

```rust
// Step 1 - collect all IDAT chunks
ChunkType::IDAT => compressed_data.extend_from_slice(&chunk.data),
```

```rust
// Step 2 - decompress the whole buffer at once
let mut decoder = ZlibDecoder::new(&compressed_data[..]);
let mut decompressed: Vec<u8> = Vec::new();
decoder.read_to_end(&mut decompressed).unwrap();
```

> The result is not raw pixels yet - it's a series of **filtered scanlines**.

---

## 6. Filtering

Before compression, PNG filters each row to make the data more compressible. After decompression you must **reverse** this step.

Each row in the decompressed data is prefixed by a **1-byte filter type**:

```
stride = 1 + (width × bytes_per_pixel)
```

The filter uses three neighbors of the current byte `x`:

```
c  b
a  x   ← x is the current filtered byte
```

| Filter | Formula |
|--------|---------|
| 0 None    | `x` |
| 1 Sub     | `x + a` |
| 2 Up      | `x + b` |
| 3 Average | `x + floor((a + b) / 2)` |
| 4 Paeth   | `x + paeth_predictor(a, b, c)` |

> All additions use `wrapping_add` - overflow is intentional.

```rust
fn reconstruct_scanline(
    filtered_scanline: &[u8],
    previous_row: &[u8],
    width: usize,
    bytes_per_pixel: usize,
) -> Vec<u8> {
    let mut raw_pixels = vec![0u8; filtered_scanline.len()];
    let filter_type = filtered_scanline[0];

    for pos in 1..filtered_scanline.len() {
        let x = filtered_scanline[pos];

        let a = if pos > bytes_per_pixel { raw_pixels[pos - bytes_per_pixel] } else { 0 };
        let b = if !previous_row.is_empty() { previous_row[pos] } else { 0 };
        let c = if !previous_row.is_empty() && pos >= bytes_per_pixel {
            previous_row[pos - bytes_per_pixel]
        } else { 0 };

        raw_pixels[pos] = match filter_type {
            0 => x,
            1 => x.wrapping_add(a),
            2 => x.wrapping_add(b),
            3 => x.wrapping_add(((a as u32 + b as u32) / 2) as u8),
            4 => x.wrapping_add(paeth_predictor(a, b, c)),
            _ => x,
        };
    }
    raw_pixels
}
```

After reconstructing each row, strip the filter byte and store the result as `previous_row` for the next iteration:

```rust
raw_pixels.extend_from_slice(&recon_scanline[1..]);
previous_row = recon_scanline; // must be reconstructed, not filtered
```

---

## 7. IEND

Always the last chunk, always empty. Just signals the end of the file.

```rust
ChunkType::IEND => break,
```
