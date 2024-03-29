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
pub struct InvalidPNGFormat;

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
    time_chunk: Option<PNGChunk>,
    chunks: Vec<PNGChunk>,
}

pub struct PNGChunk {
    length: u32,
    chunk_type: [u8; 4],
    data: Vec<u8>,
    crc: [u8; 4],
}

// IHDR chunk
pub struct IHDRData {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

// tIME chunk
pub struct TimeData {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl PNGFile {

    pub fn from_file(filename: &str) -> Result<PNGFile, Box<dyn Error>> {
        let mut header: [u8; 8] = [0; 8];
        let mut file = File::open(filename)?;
        file.read(&mut header)?;

        // All PNG files must have the same header by definition.
        if !header.iter().zip(PNG_HEADER.iter()).all(|(a, b)| a == b) {
            return Err(InvalidPNGFormat.into());
        }

        let mut ihdr_chunk: Option<PNGChunk> = None;
        let mut time_chunk: Option<PNGChunk> = None;
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

            let chunk = PNGChunk {
                length,
                chunk_type,
                data,
                crc,
            };
            let chunk_type_str = str::from_utf8(&chunk_type).unwrap();

            if chunk_type_str == "IHDR" {
                ihdr_chunk = Some(chunk);
                continue;
            } else if chunk_type_str == "tIME" {
                time_chunk = Some(chunk);
                continue;
            }

            if chunk_type_str == "IEND" {
                found_iend = true;
            }

            chunks.push(chunk);
        }

        if let Some(ihdr) = ihdr_chunk {
            return Ok( PNGFile{ ihdr_chunk: ihdr, time_chunk, chunks } );
        }
        Err(InvalidPNGFormat.into())
    }

    pub fn get_ihdr_chunk(&self) -> &PNGChunk {
        // TODO - A caller would be more likely to care about the IHDR data, not the chunk. Change
        // this to return an IHDRData chunk. For now this won't be a struct that affects the file
        // itself, but that's probably a good future step.
        &self.ihdr_chunk
    }

    pub fn get_last_modified(&self) -> Option<TimeData> {
        // TODO Add a set_last_modified - unlike other chunks, the existing data for last time
        // modified should be entirely replaced with a new TimeData, not edited.
        if let Some(chunk) = &self.time_chunk {
            let year = u16::from_be_bytes(chunk.data[0..2].try_into().unwrap());
            let month = chunk.data[2];
            let day = chunk.data[3];
            let hour = chunk.data[4];
            let minute = chunk.data[5];
            let second = chunk.data[6];

            return Some(TimeData {
                year,
                month,
                day,
                hour,
                minute,
                second,
            });
        }
        None
    }

    pub fn get_chunks(&self) -> &Vec<PNGChunk> {
        &self.chunks
    }

    pub fn write(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let mut buffer = File::create(filename).unwrap();
        buffer.write(&PNG_HEADER)?;

        &self.ihdr_chunk.write_to_file(&mut buffer)?;
        // TODO Update to use a current timestamp since the file is being written out.
        // The spec allows the time chunk to come in this order, but it may be valuable in the
        // future to preserve the original ordering if there is one.
        if let Some(time_chunk) = &self.time_chunk {
            time_chunk.write_to_file(&mut buffer)?;
        }

        for chunk in &self.chunks {
            &chunk.write_to_file(&mut buffer)?;
        }

        Ok(())
    }
}

impl PNGChunk {
    fn write_to_file(&self, open_file: &mut File) -> Result<(), Box<dyn Error>> {
        open_file.write(&self.length.to_be_bytes())?;
        open_file.write(&self.chunk_type)?;
        open_file.write(&self.data)?;
        open_file.write(&self.crc)?;

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

impl fmt::Display for TimeData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}-{}-{} {}:{}:{}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}
