use flate2::read::ZlibDecoder;
use image::{ImageBuffer, ImageResult, Rgba};
use std::{fs, io::Read};

struct PngDecoder {
    data: Vec<u8>,
    idx: usize,
}

fn save_reconstructed_as_png(
    raw_pixel_data: &[u8],
    width: usize,
    height: usize,
    output_path: &str,
) -> ImageResult<()> {
    let img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, raw_pixel_data.to_vec())
            .expect("Raw data length doesn't match width × height × 4");

    img_buffer.save(output_path)
}
#[derive(Debug)]
struct Chunk {
    length: u32,
    typ: ChunkType,
    data: Vec<u8>,
    crc: Vec<u8>,
}

#[derive(Debug)]
enum ChunkType {
    IHDR,
    IDAT,
    IEND,
    Unknown(String),
}

impl ChunkType {
    fn from_str(s: &str) -> Self {
        match s {
            "IHDR" => ChunkType::IHDR,
            "IDAT" => ChunkType::IDAT,
            "IEND" => ChunkType::IEND,
            other => ChunkType::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug)]
struct IHDR {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

impl IHDR {
    fn new(data: Vec<u8>) -> Self {
        let width = u32::from_be_bytes(data[..4].try_into().unwrap());
        let height = u32::from_be_bytes(data[4..8].try_into().unwrap());
        let bit_depth = data[8];
        let color_type = data[9];
        let compression_method = data[10];
        let filter_method = data[11];
        let interlace_method = data[12];

        Self {
            width,
            height,
            bit_depth,
            color_type,
            compression_method,
            filter_method,
            interlace_method,
        }
    }
}

impl PngDecoder {
    fn new(data: Vec<u8>) -> Self {
        if !Self::is_png(&data) {
            panic!("Not Png")
        }
        Self { data, idx: 8 }
    }

    fn get_chunk(&mut self) -> Chunk {
        let length_bytes = self.move_to(4);
        let length = u32::from_be_bytes(length_bytes.try_into().unwrap());
        let typ = String::from_utf8(self.move_to(4)).unwrap();
        let typ = ChunkType::from_str(&typ);
        let data = self.move_to(length as usize);
        let crc = self.move_to(4);
        Chunk {
            length,
            typ,
            data,
            crc,
        }
    }

    fn move_to(&mut self, len: usize) -> Vec<u8> {
        let slice = self.data[self.idx..self.idx + len].to_vec();
        self.idx += len;
        slice
    }

    fn is_png(data: &[u8]) -> bool {
        if data.len() < 8 {
            panic!("not a ping")
        }

        let png_signature = [137, 80, 78, 71, 13, 10, 26, 10];
        let file_signature = &data[..8];
        file_signature == png_signature
    }
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    match (pa, pb, pc) {
        (pa, pb, pc) if pa <= pb && pa <= pc => a,
        (pa, pb, pc) if pb <= pc => b,
        _ => c,
    }
}
fn main() {
    let data = fs::read("image.png").unwrap();
    let mut decoder = PngDecoder::new(data);
    let mut compressed_data: Vec<u8> = Vec::new();
    let mut width: usize = 0;
    let mut height: usize = 0;
    loop {
        let chunk = decoder.get_chunk();
        match chunk.typ {
            ChunkType::IHDR => {
                let ihdr = IHDR::new(chunk.data);
                println!("{}", ihdr.color_type);
                width = ihdr.width as usize;
                height = ihdr.height as usize;
            }
            ChunkType::IDAT => {
                compressed_data.extend_from_slice(&chunk.data);
            }
            ChunkType::IEND => {
                break;
            }
            ChunkType::Unknown(e) => {}
        }
    }
    //  println!("{:#?}", &compressed_data);

    let mut zlib_decoder = ZlibDecoder::new(&compressed_data[..]);
    let mut decompressed_data: Vec<u8> = Vec::new();
    zlib_decoder.read_to_end(&mut decompressed_data).unwrap();
    // println!("{:#?}", &decompressed_data.len());

    let bytes_per_pixel = 4;
    let stride = 1 + width * bytes_per_pixel;
    let mut raw_pixels: Vec<u8> = Vec::with_capacity(decompressed_data.len());
    let mut previous_row: Vec<u8> = vec![];

    for y in 0..height {
        let start = y * stride;
        let end = start + stride;
        let filtered_scanline = &decompressed_data[start..end];
        let filter_type = filtered_scanline[0];
        println!("{:?}", filter_type);

        let recon_scanline =
            reconstruct_scanline(filtered_scanline, &previous_row, width, bytes_per_pixel);

        raw_pixels.extend_from_slice(&recon_scanline[1..]);
        previous_row = recon_scanline;
    }

    save_reconstructed_as_png(&raw_pixels, width, height, "reconstructed.png")
        .expect("Failed to save reconstructed image");
}

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

        let a = if pos > bytes_per_pixel {
            raw_pixels[pos - bytes_per_pixel]
        } else {
            0
        };

        let b = if !previous_row.is_empty() && pos < previous_row.len() {
            previous_row[pos]
        } else {
            0
        };

        let c = if !previous_row.is_empty() && pos >= bytes_per_pixel {
            previous_row[pos - bytes_per_pixel]
        } else {
            0
        };

        println!("{}", a);
        let recon = match filter_type {
            0 => x,
            1 => x.wrapping_add(a),
            2 => x.wrapping_add(b),
            3 => {
                let avg = ((a as u32 + b as u32) / 2) as u8;
                x.wrapping_add(avg)
            }
            4 => x.wrapping_add(paeth_predictor(a, b, c)),
            _ => x,
        };

        raw_pixels[pos] = recon;
    }
    raw_pixels
}
