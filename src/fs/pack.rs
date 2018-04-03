// Copyright (C) 1996-1997 Id Software, Inc.
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; either version 2
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
//
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, write to the Free Software
// Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.

// Modified by Adrian Chan, March 2018
// Pulled the pack handling out of common.c

//! Things related to working with .pak files.

use std::fs::File;
use std::io::BufReader;
use std::ops::Range;
use std::path::{Path, PathBuf};

use failure::Error;
use try_from_temp::TryFromTemp;


/// Represents a .pak file -- a bundle of other files, a bit like a .tar file.
#[derive(Debug)]
pub struct Pack {
    /// The location of this .pak file.
    file_path: PathBuf,
    /// An open reader.
    reader: BufReader<File>,
    /// Information about each file contained within the pack.
    file_infos: Vec<FileInfo>,
}

const MAX_FILES_IN_PACK: usize = 2048;

impl Pack {
    /// Attempt to load the given .pak file.
    ///
    /// If no file exists at the given path, None is returned.
    /// Otherwise, an attempt is made to read the file and parse its contents.
    /// Any problems while doing this result in an `Err(Error)` being
    /// returned.
    pub fn load(path: PathBuf) -> Result<Option<Self>, Error> {
        use std::io::{Seek, SeekFrom};

        let file = match File::open(&path) {
            Err(_) => return Ok(None),
            // If file doesn't exist it's not an error.
            Ok(file) => file
        };
        let mut reader = BufReader::new(file);

        let header = PackHeader::from(&mut reader)?;
        let num_pack_files = header.dir_len / FILE_INFO_SIZE_ON_DISK;
        if num_pack_files > MAX_FILES_IN_PACK {
            bail!("too many files ({}) in pack", num_pack_files);
        }

        let mut file_infos = Vec::with_capacity(num_pack_files);
        reader.seek(SeekFrom::Start(header.dir_offset))?;
        for _i in 0..num_pack_files {
            let info = FileInfo::from(&mut reader)?;
            file_infos.push(info);
        }
        // Skip the CRC check, I don't care whether it was modified or not.

        Ok(Some(Self {
            file_path: path,
            reader,
            file_infos,
        }))
    }

    /// The location of this .pak file on disk.
    pub fn path(&self) -> &Path {
        self.file_path.as_path()
    }
}

/// Represents the `Pack`'s header information.
#[derive(Debug)]
struct PackHeader {
    // id: [u8; 4],  part of the header on disk, but we don't need to store it
    /// Offset of the file information table within the pack.
    dir_offset: u64,  // i32 on disk
    /// Size of the file infomation table, in bytes.
    dir_len: usize,  // i32 on disk
}

impl PackHeader {
    /// Parse the `PackHeader` from a reader at its current position.

    // TODO: switch to impl trait?
    fn from(reader: &mut BufReader<File>) -> Result<Self, Error> {
        use std::io::Read;
        use byteorder::{ByteOrder, LittleEndian};

        const HEADER_SIZE: usize = 4+4+4;
        const ID_SLICE: Range<usize> = 0..4;
        const DIR_OFFSET_SLICE: Range<usize> = 4..8;
        const DIR_LEN_SLICE: Range<usize> = 8..12;

        let mut header = [0; HEADER_SIZE];
        reader.read_exact(&mut header)?;

        if &header[ID_SLICE] != b"PACK" {
            bail!("Not a packfile header");
        }

        let dir_offset = u64::try_from_temp(
            LittleEndian::read_i32(&header[DIR_OFFSET_SLICE]))?;
        let dir_len = usize::try_from_temp(
            LittleEndian::read_i32(&header[DIR_LEN_SLICE]))?;

        Ok(Self {
            dir_offset,
            dir_len,
        })
    }
}

/// Information about a file stored within a `Pack`.
#[derive(Debug)]
pub struct FileInfo {
    // name: [u8; 56],  file name is stored as 56 char on disk, but we'll store
    // it in memory as a String.
    name: String,
    /// Offset of the start of the file within the pack.
    offset: u64,  // i32 on disk
    /// Size of the file, in bytes.
    size: usize,  // i32 on disk
}

const FILE_INFO_SIZE_ON_DISK: usize = 56+4+4;

impl FileInfo {
    /// Parse the `FileInfo` from a reader at its current position.

    // TODO: switch to impl trait?
    fn from(reader: &mut BufReader<File>) -> Result<Self, Error> {
        use std::ffi::CStr;
        use std::io::Read;
        use byteorder::{ByteOrder, LittleEndian};

        const NAME_SLICE: Range<usize> = 0..56;
        const OFFSET_SLICE: Range<usize> = 56..60;
        const SIZE_SLICE: Range<usize> = 60..64;

        let mut info = [0; FILE_INFO_SIZE_ON_DISK];
        reader.read_exact(&mut info)?;

        // NAME_SLICE is the max length slice of bytes that could contain the
        // name, and will normally contain many trailing zeros.
        // from_bytes_with_nul() only accepts one trailing zero, so attempt to
        // find the subset of the slice that satisfies this.
        let name_slice = match info[NAME_SLICE].iter().position(|n| n == &0) {
            Some(i) => 0..(i+1),
            None => NAME_SLICE
        };
        let c_name = CStr::from_bytes_with_nul(&info[name_slice])?;
        let name = c_name.to_str()?.to_string();
        let offset = u64::try_from_temp(
            LittleEndian::read_i32(&info[OFFSET_SLICE]))?;
        let size = usize::try_from_temp(
            LittleEndian::read_i32(&info[SIZE_SLICE]))?;

        Ok(Self {
            name,
            offset,
            size,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test_common as common;

    #[test]
    fn read_pak0() {
        let path = common::pak0_path();
        let mut pack = Pack::load(path).unwrap().unwrap();
        assert_eq!(pack.file_infos.len(), 339);

        let f = &pack.file_infos[0];
        assert_eq!(f.name, "sound/items/r_item1.wav");
        assert_eq!(f.offset, 12);
        assert_eq!(f.size, 6822);

        // Simple check: does it look like there's a WAV file at the offset
        // given?
        use std::io::{Read, Seek, SeekFrom};
        let mut wav_start = vec![0; 4];
        pack.reader.seek(SeekFrom::Start(f.offset)).unwrap();
        pack.reader.read_exact(&mut wav_start).unwrap();
        assert_eq!(wav_start, b"RIFF");
    }
}
