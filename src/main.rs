use flate2::read::ZlibDecoder;
use std::{fs, io::Read};

struct PngDecoder {
    data: Vec<u8>,
    idx: usize,
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

fn main() {
    let data = fs::read("image.png").unwrap();
    let mut decoder = PngDecoder::new(data);
    let mut compressed_data: Vec<u8> = Vec::new();

    loop {
        let chunk = decoder.get_chunk();
        match chunk.typ {
            ChunkType::IHDR => {
                let ihdr = IHDR::new(chunk.data);
                println!("{:#?}", ihdr);
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
    println!("{:#?}", &decompressed_data);
}
