# Portable Network Graphics (PNG)

## How To Create Your Own Decoder

### Know The Png Structure
  1. **PNG** file signature
  > The first 8 bytes of a PNG file must be `137 80 78 71 13 10 26 10`.
  ```rust
use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let mut file = File::open("image.png")?;
    let mut signature = [0u8; 8];
    file.read_exact(&mut signature)?;

    if signature != [137, 80, 78, 71, 13, 10, 26, 10] {
        println!("Not a PNG");
        return Ok(());
    }
    println!("Valid PNG signature: {:?}", signature);
    Ok(())
}
```


  2. **Chunk layout**
  Each PNG chunk consists of four main parts:
    1. **Length**
      The number of bytes in the **Chunk Data** field.
      This field is **4 bytes** long and stored as an **unsigned 32-bit integer** in **big-endian** format. 
       It does not include the size of the Chunk Type or the CRC.
  
    2. Chunk Type
      Identifies the type of the chunk.  
      It is **4 bytes** long and consists of **ASCII characters**, for example:
      
      PNG chunks are divided into two categories:
      - **Critical chunks**:  
        - `IHDR`
        - `IDAT`
        - `IEND`
        These chunks are required to properly decode the image.  
        A PNG decoder **must understand and process** these chunks.
   
      - **Ancillary chunks**:  
        
      For simplicity, this decoder will only process **critical chunks** and ignore **ancillary chunks**.
        
    3. Chunk Data
        The data bytes appropriate to the chunk type, if any. This field can be of zero length.
        
    4. CRC
         **CRC (Cyclic Redundancy Check)**  
            A **4-byte** value used to detect errors in the chunk data.  
            It is calculated using the **Chunk Type** and **Chunk Data** fields, but **not the Length field**.
            The CRC is used to check if the chunk data is correct and not corrupted.

## Critical chunks
  1. IHDR Image header
    The IHDR chunk must appear FIRST.  It contains:
      Width:              4 bytes
      Height:             4 bytes
      Bit depth:          1 byte
      Color type:         1 byte
      Compression method: 1 byte
      Filter method:      1 byte
      Interlace method:   1 byte

  <!--let's read a chunk-->
