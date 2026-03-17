# Portable Network Graphics (PNG)
## How To Build Your Own Decoder in Rust
 
---
 
## 1. File Signature
 
Every PNG file starts with the same 8 magic bytes. This is the first thing you check if it doesn't match, the file is not a valid PNG.
 
```rust
use std::fs;
 
fn main() -> std::io::Result<()> {
    let data = fs::read("image.png")?;
 
    let signature = &data[0..8];
    if signature != [137, 80, 78, 71, 13, 10, 26, 10] {
        eprintln!("Not a valid PNG file.");
        return Ok(());
    }
 
    println!("Valid PNG signature!");
    Ok(())
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
 
The case of the first letter in the Type field tells you if a chunk is critical or optional:
 
| Category  | First Letter | Examples           | Meaning |
|-----------|--------------|--------------------|---------|
| Critical  | Uppercase    | `IHDR`, `IDAT`, `IEND` | Must be understood to decode the image |
| Ancillary | Lowercase    | `tEXt`, `gAMA`, `tIME` | Safe to ignore if unrecognised |
 
For our decoder we only process `IHDR`, `IDAT`, and `IEND`.
 
---

---
 
## 3. Parsing All Chunks
 
The first chunk starts at byte index `8` (right after the signature). We loop, reading chunk after chunk, until we hit `IEND`.
 
```rust
fn parse_chunks(data: &[u8]) {
    let mut idx = 8; // skip 8-byte signature
 
    loop {
        // --- Length (4 bytes, big-endian) ---
        let length = u32::from_be_bytes(data[idx..idx + 4].try_into().unwrap()) as usize;
        idx += 4;
 
        // --- Type (4 bytes, ASCII) ---
        let chunk_type = &data[idx..idx + 4];
        let type_str = std::str::from_utf8(chunk_type).unwrap_or("????");
        idx += 4;
 
        // --- Data (length bytes) ---
        let chunk_data = &data[idx..idx + length];
        idx += length;
 
        // --- CRC (4 bytes) ---
        let _crc = &data[idx..idx + 4];
        idx += 4;
 
        println!("Chunk: {} | {} bytes", type_str, length);
 
        match type_str {
            "IHDR" => handle_ihdr(chunk_data),
            "IDAT" => handle_idat(chunk_data),
            "IEND" => {
                println!("Reached IEND — done.");
                break;
            }
            _ => {
                // Ancillary or unknown chunk — safe to skip
            }
        }
    }
}
```
 
> **Rules you must follow:**
> - The **first** chunk must always be `IHDR`.
> - The **last** chunk must always be `IEND`.
> - There can be **multiple** `IDAT` chunks all their data must be concatenated before decompression.
 
---

## 4. IHDR — Image Header
 
`IHDR` is always exactly **13 bytes** and contains everything you need to know about the image format.
 
```
Bytes  0-3  : Width              (u32, big-endian)
Bytes  4-7  : Height             (u32, big-endian)
Byte   8    : Bit depth          (u8)
Byte   9    : Color type         (u8)
Byte  10    : Compression method (u8) - always 0
Byte  11    : Filter method      (u8) - always 0
Byte  12    : Interlace method   (u8) - 0 = none, 1 = Adam7
```

### Color Types
 
| Value | Name             | Channels        | Allowed Bit Depths |
|-------|------------------|-----------------|--------------------|
| 0     | Grayscale        | 1 (Gray)        | 1, 2, 4, 8, 16     |
| 2     | Truecolor        | 3 (RGB)         | 8, 16              |
| 3     | Indexed          | 1 (palette idx) | 1, 2, 4, 8         |
| 4     | Grayscale+Alpha  | 2 (Gray, A)     | 8, 16              |
| 6     | Truecolor+Alpha  | 4 (RGBA)        | 8, 16              |

```rust
#[derive(Debug)]
struct IhdrData {
    width:              u32,
    height:             u32,
    bit_depth:          u8,
    color_type:         u8,
    compression_method: u8,
    filter_method:      u8,
    interlace_method:   u8,
}
 
fn handle_ihdr(data: &[u8]) -> IhdrData {
    assert_eq!(data.len(), 13, "IHDR must be exactly 13 bytes");
 
    let ihdr = IhdrData {
        width:              u32::from_be_bytes(data[0..4].try_into().unwrap()),
        height:             u32::from_be_bytes(data[4..8].try_into().unwrap()),
        bit_depth:          data[8],
        color_type:         data[9],
        compression_method: data[10],
        filter_method:      data[11],
        interlace_method:   data[12],
    };
 
    println!("Image: {}x{} | bit_depth={} | color_type={}", 
        ihdr.width, ihdr.height, ihdr.bit_depth, ihdr.color_type);
 
    ihdr
}
```
