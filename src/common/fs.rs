use std;
use std::ops::Range;
use std::io;
use std::io::{Read, Seek};

extern crate byteorder;
use self::byteorder::{ByteOrder, LittleEndian};

use ::parms as parms;
use ::crc as crc;


const DISK_PACK_HEADER_SIZE: usize = 4+4+4;
const PACK_HEADER_ID_RANGE: Range<usize> = 0..4;
const PACK_HEADER_DIR_OFFSET_RANGE: Range<usize> = 4..8;
const PACK_HEADER_DIR_LEN_RANGE: Range<usize> = 8..12;

const DISK_PACK_FILE_SIZE: usize = 56+4+4;
const DISK_PACK_FILE_NAME_RANGE: Range<usize> = 0..56;
const DISK_PACK_FILE_POS_RANGE: Range<usize> = 56..60;
const DISK_PACK_FILE_LEN_RANGE: Range<usize> = 60..64;

const MAX_FILES_IN_PACK: i32 = 2048;
const PAK0_COUNT: i32 = 339;
const PAK0_CRC: u16 = 32981;


pub struct FileSystem {
    search_paths: Vec<SearchPath>,
    gamedir: String,
    modified: bool,
    proghack: bool
}

pub enum SearchPath {
    Filename(String),
    Pack(Pack)
}

struct DiskPackHeader {
    // id: [u8; 4],  part of the header on disk, but we don't need to store it
    dir_offset: i32,
    dir_len: i32
}

struct DiskPackFile {
    name: [u8; 56],
    file_pos: i32,
    file_len: i32
}

struct PackFile {
    name: String,
    file_pos: i32,
    file_len: i32
}

pub struct Pack {
    filename: String,
    file_handle: std::fs::File,
    num_pack_files: i32,
    files: Vec<PackFile>
}

impl DiskPackHeader {
    fn from(file: &mut std::fs::File) -> io::Result<Self> {
        let mut header: Vec<u8> = Vec::with_capacity(DISK_PACK_HEADER_SIZE);
        file.seek(io::SeekFrom::Start(0))?;
        file.read_exact(&mut header)?;

        if &header[PACK_HEADER_ID_RANGE] != "PACK".as_bytes() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a packfile header"));
        }

        let dir_offset = LittleEndian::read_i32(
            &header[PACK_HEADER_DIR_OFFSET_RANGE]);
        let dir_len = LittleEndian::read_i32(
            &header[PACK_HEADER_DIR_LEN_RANGE]);

        Ok(Self {dir_offset, dir_len})
    }
}

impl PackFile {
    fn vec_from(info: &Vec<u8>) -> Result<Vec<PackFile>, std::str::Utf8Error> {
        let file_count = {
            let raw_len = info.len();
            assert!(raw_len % DISK_PACK_FILE_SIZE == 0);
            raw_len / DISK_PACK_FILE_SIZE
        };
        let mut pack_files = Vec::with_capacity(file_count);

        for raw_file in info.chunks(DISK_PACK_FILE_SIZE) {
            pack_files.push(PackFile::from(&raw_file)?);
        }

        Ok(pack_files)
    }

    fn from(raw: &[u8]) -> Result<PackFile, std::str::Utf8Error> {
        assert_eq!(raw.len(), DISK_PACK_FILE_SIZE);
        let name = String::from(
            std::str::from_utf8(&raw[DISK_PACK_FILE_NAME_RANGE])?
        );
        let file_pos = LittleEndian::read_i32(&raw[DISK_PACK_FILE_POS_RANGE]);
        let file_len = LittleEndian::read_i32(&raw[DISK_PACK_FILE_LEN_RANGE]);
        Ok(PackFile {name, file_pos, file_len})
    }
}

pub fn new(parms: &parms::Parms) -> FileSystem {
    let basedir: String = {
        if let Some(basedir_override) = parms.get_parm_value::<String>("-basedir") {
            basedir_override
        } else {
            parms.cwd.clone()
        }
    };

    if let Some(_cachedir_override) = parms.get_parm_value::<String>("-cachedir") {
        unimplemented!("-cachedir not supported");
    }

    let mut fs = FileSystem {
        search_paths: Vec::with_capacity(10),
        gamedir: String::new(),
        modified: false,
        proghack: parms.has_parm("-proghack")
    };

    let make_path = |tail: &str| {
        let mut path = std::path::PathBuf::from(&basedir);
        path.push(tail);
        path
    };

    fs.add_game_directory(make_path("id1"));
    
    if parms.rogue {
        fs.add_game_directory(make_path("rogue"));
    }
    if parms.hipnotic {
        fs.add_game_directory(make_path("hipnotic"));
    }

    if let Some(game_override) = parms.get_parm_value::<String>("-game") {
        fs.modified = true;
        fs.add_game_directory(make_path(&game_override));
    }

    if let Some(raw_vals) = parms.get_raw_parm_values("-path") {
        fs.modified = true;
        fs.search_paths.clear();
        for file_or_dir_name in raw_vals {
            if file_extension(&file_or_dir_name) == "pak" {
                let _pack = fs.load_pack_file(&file_or_dir_name);
                unimplemented!();
            } else {
                let file = SearchPath::Filename(file_or_dir_name);
                fs.search_paths.push(file);
            }
        }
    }

    fs
}

pub fn file_extension(filename: &str) -> String {
    let path = std::path::Path::new(&filename);
    String::from(
        path.extension().unwrap_or(std::ffi::OsStr::new(""))
        .to_string_lossy())
}

impl FileSystem {
    fn add_game_directory<P>(&mut self, path: P)
        where P : AsRef<std::path::Path>
    {
        let path = path.as_ref();
        self.gamedir = String::from(path.to_string_lossy());
        self.search_paths.insert(
            0, SearchPath::Filename(String::from(path.to_string_lossy())));

        for i in 0.. {
            let mut pakfile = std::path::PathBuf::from(path);
            pakfile.push(format!("pak{}.pak", i));

            if let Some(pak) = self.load_pack_file(pakfile) {
                self.search_paths.insert(0, pak);
            } else {
                break;
            }
        }
    }

    fn load_pack_file<P>(&mut self, path: P) -> Option<SearchPath>
        where P: AsRef<std::path::Path>
    {
        let mut file = match std::fs::File::open(&path) {
            Err(_) => return None,
            Ok(file) => file
        };

        let header = match DiskPackHeader::from(&mut file) {
            Ok(h) => h,
            Err(e) => unimplemented!("sys_error() invalid packfile: {:?}", e)
        };
        let num_pack_files = header.dir_len / DISK_PACK_FILE_SIZE as i32;

        if num_pack_files > MAX_FILES_IN_PACK {
            unimplemented!("sys_error() too many files in pack");
        }
        if num_pack_files != PAK0_COUNT {
            // Not the original file.
            self.modified = true;
        }

        let mut dir_info: Vec<u8> = Vec::with_capacity(header.dir_len as usize);
        file.seek(io::SeekFrom::Start(header.dir_offset as u64));
        if let Err(_) = file.read_exact(&mut dir_info) {
            unimplemented!("sys_error() pack's dir info is the wrong size");
        }

        // CRC the directory to check for modifications.
        let mut crc = crc::Crc::new();
        for byte in &dir_info {
            crc.process_byte(*byte);
        }
        if crc.crc_value() != PAK0_CRC {
            self.modified = true;
        }

        // Parse the directory.
        let pack_files = match PackFile::vec_from(&dir_info) {
            Ok(pf) => pf,
            Err(e) => unimplemented!("sys_error() invalid packfile: {:?}", e)
        };

        // TODO: con_printf(
        //    "Added packfile {} ({} files)", path, num_pack_files);
        let pack = Pack {
            filename: String::from(path.as_ref().to_string_lossy()),
            file_handle: file,
            num_pack_files,
            files: pack_files
        };
        Some(SearchPath::Pack(pack))
    }
}
