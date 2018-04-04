//! Misc utility functions.

use std::ffi::CStr;

use failure::Error;


/// Try to read a `String` from a `u8` buffer that contains a C-style string.
///
/// We often have a fixed-size buffer that contains a C string, where the
/// string does not fill the entire buffer.  In this case, the remainder of the
/// buffer may be filled with zeroes, or junk bytes -- in either case, a
/// naive `CStr::from_bytes_with_nul()` will fail due to the extra data after
/// the nul terminator.
///
/// This function will attempt the conversion, but ignore all data after the
/// first nul byte.
pub fn cstr_buf_to_string(buf: &[u8]) -> Result<String, Error> {
    // Attempt to find the subset of the buffer that contains a single nul byte.
    // If there is no nul byte, use the entire buffer and let the error be
    // handled by `CStr::from_bytes_with_nul()`.
    let cstr_slice = match buf.iter().position(|c| c == &b'\0') {
        Some(i) => 0..(i+1),
        None => 0..buf.len()
    };
    let cstr = CStr::from_bytes_with_nul(&buf[cstr_slice])?;
    Ok(cstr.to_str()?.to_string())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cstr_buf_simple() {
        let buf = b"abc\0";
        assert_eq!(cstr_buf_to_string(&buf[..]).unwrap(), "abc");
    }

    #[test]
    fn cstr_buf_with_trailing_junk() {
        {
            let buf = b"abc\0\0";
            assert_eq!(cstr_buf_to_string(&buf[..]).unwrap(), "abc");
        }

        {
            let buf = b"abc\0dh29834";
            assert_eq!(cstr_buf_to_string(&buf[..]).unwrap(), "abc");
        }

        {
            let buf = b"abc\0dh29834\0";
            assert_eq!(cstr_buf_to_string(&buf[..]).unwrap(), "abc");
        }
    }

    #[test]
    fn cstr_buf_terminator_only() {
        let buf = b"\0";
        assert_eq!(cstr_buf_to_string(&buf[..]).unwrap(), "");
    }

    #[test]
    fn cstr_buf_no_terminator() {
        {
            let buf = b"abc";
            match cstr_buf_to_string(&buf[..]) {
                Err(_) => (),
                Ok(s) => panic!("expected an error, got '{}'", s),
            }
        }

        {
            let buf = b"";
            match cstr_buf_to_string(&buf[..]) {
                Err(_) => (),
                Ok(s) => panic!("expected an error, got '{}'", s),
            }
        }
    }
}
