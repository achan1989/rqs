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

//! Work with wad files.
//!
//! A wad file is another tar-like container file.
//! It contains multiple "lumps" (essentially a file's data) and their
//! associated metadata (`LumpInfo`s).
//!
//! This metadata includes a lump's name, what kind of data it contains (see
//! `LumpType`), whether the data is compressed, etc.

use std::ops::Range;

use byteorder::{ByteOrder, LittleEndian};
use failure::Error;

use fs::FileSys;
use try_from_temp::TryFromTemp;
use util;


/// A wad file loaded into memory.
#[derive(Debug)]
pub struct Wad {
    data: Vec<u8>,
    info: WadInfo,
    lumps: Vec<LumpInfo>,
}

impl Wad {
    /// Load a wad file from the filesystem.
    pub fn load_from_file(file_name: &str, fs: &mut FileSys)
        -> Result<Self, Error>
    {
        let mut data =
            fs.load_file(file_name)?
            .ok_or_else(
                || format_err!("no such file {}", file_name))?;
        let info = WadInfo::from(&data)?;
        let mut lumps = Vec::with_capacity(info.num_lumps);

        let make_lump_info_slice = |i| {
            info.lump_table_offset + (i * LUMP_INFO_SIZE)
            ..
            info.lump_table_offset + (i * LUMP_INFO_SIZE) + LUMP_INFO_SIZE
        };

        for i in 0..info.num_lumps {
            let mut lump_info_buf = &mut data[make_lump_info_slice(i)];
            let lump_info = LumpInfo::from(lump_info_buf)?;
            // We won't bother to lowercase the names, since we'll just use
            // `eq_ignore_ascii_case()` when making the comparisons.
            // And we won't implement `SwapPic` either -- we'll interpret the
            // lump data when it's used.
            lumps.push(lump_info);
        }
        assert_eq!(lumps.len(), info.num_lumps);

        Ok(Self {
            data,
            info,
            lumps,
        })
    }

    /// Try to get information about the lump with the given name.
    pub fn lump_info(&self, name: &str) -> Result<&LumpInfo, Error> {
        self.lumps.iter()
            .find(|l| l.name.eq_ignore_ascii_case(name))
            .ok_or_else(
                || format_err!("No lump named {}", name))
    }

    /// Try to get the data from the lump with the given name.
    ///
    /// **Warning:** this does not uncompress the data.
    pub fn data_for_lump_named(&self, name: &str) -> Result<&[u8], Error> {
        let lump = self.lump_info(name)?;
        Ok(&self.data[lump.file_pos..lump.file_pos+lump.disk_size])
    }

    /// Try to get the data from the lump with the given number.
    ///
    /// **Warning:** this does not uncompress the data.
    pub fn data_for_lump_num(&self, n: usize) -> Result<&[u8], Error> {
        if n > self.info.num_lumps {
            bail!("Bad lump number: {}", n);
        }
        let lump = &self.lumps[n];
        Ok(&self.data[lump.file_pos..lump.file_pos+lump.disk_size])
    }
}

#[derive(Clone, Copy, Debug)]
struct WadInfo {
    num_lumps: usize,
    lump_table_offset: usize,
}

impl WadInfo {
    fn from(data: &[u8]) -> Result<Self, Error> {
        const HEADER_SIZE: usize = 4+4+4;
        const HEADER_SLICE: Range<usize> = 0..4;
        const NUM_LUMPS_SLICE: Range<usize> = 4..8;
        const OFFSET_SLICE: Range<usize> = 8..12;

        if data.len() < HEADER_SIZE {
            bail!("Invalid wad header: too short");
        }
        if &data[HEADER_SLICE] != b"WAD2" {
            bail!("Invalid wad header: no WAD2 id");
        }

        let num_lumps = usize::try_from_temp(
            LittleEndian::read_i32(&data[NUM_LUMPS_SLICE]))?;
        let lump_table_offset = usize::try_from_temp(
            LittleEndian::read_i32(&data[OFFSET_SLICE]))?;
        Ok(Self {
            num_lumps,
            lump_table_offset,
        })
    }
}

/// Information about a lump.
#[derive(Debug)]
pub struct LumpInfo {
    file_pos: usize,
    /// Size of the data on disk, in bytes.
    disk_size: usize,
    /// Size of the data once uncompressed, in bytes.
    size: usize,
    lump_type: LumpType,
    compression: Compression,
    // name: [u8; 16],  lump name is stored as 16 char on disk, but we'll store
    // it in memory as...
    name: String,
}

const LUMP_INFO_SIZE: usize = 4+4+4+1+1+1+1+16;

impl LumpInfo {
    fn from(data: &[u8]) -> Result<Self, Error> {
        if data.len() != LUMP_INFO_SIZE {
            bail!("Invalid lump info size: expected {} got {}",
                  LUMP_INFO_SIZE, data.len());
        }

        const FILE_POS_SLICE: Range<usize> = 0..4;
        const DISK_SIZE_SLICE: Range<usize> = 4..8;
        const UNCOMPRESSED_SIZE_SLICE: Range<usize> = 8..12;
        const TYPE_INDEX: usize = 12;
        const COMPRESSION_INDEX: usize = 13;
        // Skip 2 padding bytes.
        const NAME_SLICE: Range<usize> = 16..32;

        let file_pos = usize::try_from_temp(
            LittleEndian::read_i32(&data[FILE_POS_SLICE]))?;
        let disk_size = usize::try_from_temp(
            LittleEndian::read_i32(&data[DISK_SIZE_SLICE]))?;
        let size = usize::try_from_temp(
            LittleEndian::read_i32(&data[UNCOMPRESSED_SIZE_SLICE]))?;
        let lump_type = LumpType::try_from(
            data[TYPE_INDEX])?;
        let compression = Compression::try_from(
            data[COMPRESSION_INDEX])?;
        let name = util::cstr_buf_to_string(&data[NAME_SLICE])?;

        Ok(Self {
            file_pos,
            disk_size,
            size,
            lump_type,
            compression,
            name,
        })
    }

    /// The number of bytes that this lump occupies on disk.
    pub fn disk_size(&self) -> usize {
        self.disk_size
    }

    /// The number of bytes that this lump will occupy in memory, once
    /// decompressed.  May be the same as the `disk_size`.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The kind of data that the lump contains.
    pub fn lump_type(&self) -> LumpType {
        self.lump_type
    }

    /// The kind of compression the lump is using, if any.
    pub fn compression(&self) -> Compression {
        self.compression
    }

    /// The name of the lump.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The kind of compression that has been applied to a lump.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Compression {
    /// No compression.
    None,
    /// [LZSS compression](https://wikipedia.org/wiki/Lempel-Ziv-Storer-Szymanski).
    Lzss,
}

impl Compression {
    fn try_from(n: u8) -> Result<Self, Error> {
        match n {
            0 => Ok(Compression::None),
            1 => Ok(Compression::Lzss),
            _ => Err(format_err!("Invalid compression type: {}", n)),
        }
    }
}

/// The kind of data that a lump contains.
///
/// The exact meaning and use case of each variant is still TBD.
/// Note that a `Lumpy` variant is currently missing; it shares the same
/// numeric code as the `Palette` variant, and seems to be a legacy type.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LumpType {
    None,
    Label,
    // Lumpy, defined to be the same as Palette. Just ignore for now.
    Palette,
    Qtex,
    Qpic,
    Sound,
    Miptex,
}

impl LumpType {
    fn try_from(n: u8) -> Result<Self, Error> {
        match n {
            0 => Ok(LumpType::None),
            1 => Ok(LumpType::Label),
            64 => Ok(LumpType::Palette),
            65 => Ok(LumpType::Qtex),
            66 => Ok(LumpType::Qpic),
            67 => Ok(LumpType::Sound),
            68 => Ok(LumpType::Miptex),
            _ => Err(format_err!("Invalid lump type: {}", n))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use parms::Parms;
    use test_common as common;

    // These tests make use of a FileSys of a full version of standard Quake 1.

    /// Just see if we can load something from a wad.
    #[test]
    fn wad_load() {
        let parms = Parms::new(
            vec!["-basedir".into(), common::base_dir().to_string_lossy().to_string()],
            "cwd".into());
        let mut fs = FileSys::new(&parms).unwrap();

        let wad = Wad::load_from_file("gfx.wad", &mut fs).unwrap();

        // This is the "loading" icon.
        let info = wad.lump_info("disc").unwrap();
        assert_eq!(info.lump_type(), LumpType::Qpic);
        assert_eq!(info.compression(), Compression::None);

        let data = wad.data_for_lump_named("disc").unwrap();
        assert_eq!(data.len(), info.size());
    }
}
