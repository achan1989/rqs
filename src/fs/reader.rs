// Copyright (C) 2018 Adrian Chan
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

//! Functionality for reading the contents of the Quake filesystem.

use std::fs::File;
use std::io::{BufReader, Read, Result};

use fs::pack::FileInfo;


/// Represents a bounded one-time read from a file in the Quake filesystem.
///
/// The `FsReader` cannot seek, and cannot read past the end of the file.
/// This is useful because a Quake filesystem "file" may actually be a small
/// section of a larger file-on-disk.
///
/// Reads using the `FsReader` are buffered.
///
/// The `FsReader` has exclusive control of the underlying reader, so conflicts
/// with multiple seekers/readers is not possible.
pub struct FsReader<'a> {
    inner: FsReaderKind<'a>,
    len: usize,
    n_read: usize,
}

// * If we're reading a file inside a .pak file, we are only borrowing the
//   underlying reader from the `Pack`.
// * If we're reading a file from disk, we own the underlying reader.
enum FsReaderKind<'a> {
    Borrowed(&'a mut BufReader<File>),
    Owned(BufReader<File>)
}

impl<'a> FsReader<'a> {
    /// Create a new `FsReader` that reads the given file on disk.
    pub fn for_file(file: File) -> Result<Self> {
        let len = file.metadata()?.len() as usize;
        let br = BufReader::new(file);
        Ok(Self {
            inner: FsReaderKind::Owned(br),
            len,
            n_read: 0,
        })
    }

    /// Create a new `FsReader` that reads the given file within a `Pack`.
    pub fn for_pack_file(
        reader: &'a mut BufReader<File>, info: &FileInfo)
        -> Result<Self>
    {
        use std::io::{Seek, SeekFrom};

        reader.seek(SeekFrom::Start(info.offset))?;
        Ok(Self {
            inner: FsReaderKind::Borrowed(reader),
            len: info.size,
            n_read: 0,
        })
    }

    /// The size of the file, in bytes.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a> Read for FsReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let remain = self.len - self.n_read;
        if remain == 0 {
            return Ok(0);  // EOF
        }
        // Don't use all of the buffer if we don't have enough data to fill it.
        let buf = match remain < buf.len() {
            false => buf,
            true => &mut buf[0..remain],
        };

        let res = match self.inner {
            FsReaderKind::Borrowed(ref mut br) => br.read(buf),
            FsReaderKind::Owned(ref mut br) => br.read(buf),
        };
        if let Ok(n) = res {
            self.n_read += n;
        }
        res
    }
}
