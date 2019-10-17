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
    chunks: Vec<PNGChunk>,
}

pub struct PNGChunk {
    length: u32,
    chunk_type: [u8; 4],
    data: Vec<u8>,
    crc: [u8; 4],
}

impl PNGFile {
    pub fn from_file(filename: &str) -> Result<PNGFile, Box<dyn Error>> {
        let mut header: [u8; 8] = [0; 8];
        let mut png_file = File::open(filename)?;

        png_file.read(&mut header)?;

        if !header.iter().zip(PNG_HEADER.iter()).all(|(a, b)| a == b) {
            return Err(InvalidPNGFormat.into());
        }

        let chunks = get_chunks_from_file(&mut png_file);
        Ok(PNGFile { chunks })
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

fn get_chunks_from_file(file: &mut File) -> Vec<PNGChunk> {
    // This assumes the file is open and the PNG header has already been consumed from the file
    let mut chunks: Vec<PNGChunk> = Vec::new();
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

    chunks
}
