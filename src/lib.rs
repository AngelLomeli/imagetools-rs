use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::str;

use std::error;
use std::fmt;

const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Debug, Clone)]
struct InvalidPNGFormat;

impl fmt::Display for InvalidPNGFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "The provided file is not a valid PNG.")
    }
}

impl error::Error for InvalidPNGFormat {
    fn description(&self) -> &str {
        "The provided file is not a valid PNG."
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

pub struct PNGFile {
    ihdr_chunk: PNGChunk,
    chunks: Vec<PNGChunk>,
}

pub struct PNGChunk {
    length: u32,
    chunk_type: [u8; 4],
    data: Vec<u8>,
    crc: [u8; 4],
}

pub struct IHDRData {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

impl PNGFile {
    pub fn from_file(filename: &str) -> Result<PNGFile, Box<dyn Error>> {
        let mut header: [u8; 8] = [0; 8];
        let mut png_file = File::open(filename)?;

        png_file.read(&mut header)?;

        if !header.iter().zip(PNG_HEADER.iter()).all(|(a, b)| a == b) {
            return Err(InvalidPNGFormat.into());
        }

        let (ihdr_chunk, chunks) = PNGFile::get_chunks_from_file(&mut png_file);
        Ok(PNGFile { ihdr_chunk, chunks })
    }

    fn get_chunks_from_file(file: &mut File) -> (PNGChunk, Vec<PNGChunk>) {
        // This assumes the file is open and the PNG header has already been consumed from the file
        let mut chunks: Vec<PNGChunk> = Vec::new();
        let mut ihdr_chunk: Option<PNGChunk> = None;
        let mut found_iend = false;

        while !found_iend {
            let mut length: [u8; 4] = [0; 4];
            file.read(&mut length).unwrap();
            let length: u32 = u32::from_be_bytes(length);

            let mut chunk_type: [u8; 4] = [0; 4];
            file.read(&mut chunk_type).unwrap();

            let mut data: Vec<u8> = vec![0u8; length as usize];
            file.read(data.as_mut_slice()).unwrap();

            let mut crc: [u8; 4] = [0; 4];
            file.read(&mut crc).unwrap();

            if str::from_utf8(&chunk_type).unwrap() == "IHDR" {
                ihdr_chunk = Some(PNGChunk {
                    length,
                    chunk_type,
                    data,
                    crc,
                });
                continue;
            }

            if str::from_utf8(&chunk_type).unwrap() == "IEND" {
                found_iend = true;
            }

            chunks.push(PNGChunk {
                length,
                chunk_type,
                data,
                crc,
            });
        }

        if let Some(ihdr) = ihdr_chunk {
            (ihdr, chunks)
        } else {
            // TODO Use a Result as the return type or find a more elegant solution.
            panic!("No IHDR Chunk found!");
        }
    }

    pub fn get_ihdr_chunk(&self) -> &PNGChunk {
        &self.ihdr_chunk
    }

    pub fn get_chunks(&self) -> &Vec<PNGChunk> {
        &self.chunks
    }

    pub fn write(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let mut buffer = File::create(filename).unwrap();
        buffer.write(&PNG_HEADER).expect("Couldn't write to file");

        for chunk in &self.chunks {
            buffer.write(&chunk.length.to_be_bytes())?;
            buffer.write(&chunk.chunk_type)?;
            buffer.write(&chunk.data)?;
            buffer.write(&chunk.crc)?;
        }

        Ok(())
    }
}

impl fmt::Display for PNGChunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let length = &self.length;
        let chunk_type = str::from_utf8(&self.chunk_type).unwrap();
        let crc = u32::from_be_bytes(self.crc);

        let data: Vec<String> = self.data.iter().map(|b| format!("{:02X}", b)).collect();
        let data = data.join(" ");

        write!(
            f,
            "Length: {} bytes\n\
             Chunk Type: {}\n\
             CRC: {}\n\
             Data: {}",
            length, chunk_type, crc, data
        )
    }
}

impl IHDRData {
    pub fn from_chunk(chunk: &PNGChunk) -> IHDRData {
        if str::from_utf8(&chunk.chunk_type).unwrap() != "IHDR" {
            // TODO Change this to return a Result
            panic!("Not an IHDR chunk!");
        }

        let width: u32 = u32::from_be_bytes(chunk.data[0..4].try_into().unwrap());
        let height: u32 = u32::from_be_bytes(chunk.data[4..8].try_into().unwrap());
        let bit_depth = chunk.data[8];
        let color_type = chunk.data[9];
        let compression_method = chunk.data[10];
        let filter_method = chunk.data[11];
        let interlace_method = chunk.data[12];

        if width == 0 || height == 0 {
            // TODO Create error type for invalid IHDR data and return
            panic!("Width and height must be non-zero numbers.");
        }

        // TODO There's probably a cleaner way to do this. Find one or remove this comment.
        if bit_depth != 1 && bit_depth != 2 && bit_depth != 4 && bit_depth != 8 && bit_depth != 16 {
            // TODO Create error type for invalid IHDR data and return
            panic!("Invalid bit depth specified. Valid values are 1, 2, 4, 8, and 16.");
        }

        // TODO There's probably a cleaner way to do this. Find one or remove this comment.
        if color_type != 0
            && color_type != 2
            && color_type != 3
            && color_type != 4
            && color_type != 6
        {
            // TODO Create error type for invalid IHDR data and return
            panic!("Invalid color type specified. Valid values are 0, 2, 3, 4, and 6.");
        }

        if (color_type == 2 || color_type == 4 || color_type == 6)
            && (bit_depth != 8 && bit_depth != 16)
        {
            // TODO Create error type for invalid IHDR data and return
            panic!("Invalid bit depth specified for color type. Valid values are 8 and 16.");
        } else if color_type == 3 && bit_depth == 16 {
            // TODO Create error type for invalid IHDR data and return
            panic!("Invalid bit depth specified for color type. Valid values are 1, 2, 4, and 8.");
        }
        // Color type 0 allows all valid bit depths, so no check needed.

        if compression_method != 0 {
            // TODO While not defined in the ISO spec, this may still be valid. Needs more
            // research, but for now we'll reject it. Needs an IHDR error type if we don't allow
            // it.
            panic!("Unsupported compression method specified. The only valid value is 0.");
        }

        if filter_method != 0 {
            // TODO While not defined in the ISO spec, this may still be valid. Needs more
            // research, but for now we'll reject it. Needs an IHDR error type if we don't allow
            // it.
            panic!("Unsupported filter method specified. The only valid value is 0.");
        }

        if interlace_method > 1 {
            // TODO While not defined in the ISO spec, this may still be valid. Needs more
            // research, but for now we'll reject it. Needs an IHDR error type if we don't allow
            // it.
            panic!("Unsupported interlace method specified. Valid values are 0 and 1.");
        }

        // All validation passed.
        IHDRData {
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

impl fmt::Display for IHDRData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Width (pixels): {}\n\
             Height (pixels): {}\n\
             Bit depth: {}\n\
             Color Type: {}\n\
             Compression Method: {}\n\
             Filter Method: {}\n\
             Interlace Method: {}",
            self.width,
            self.height,
            self.bit_depth,
            self.color_type,
            self.compression_method,
            self.filter_method,
            self.interlace_method
        )
    }
}
