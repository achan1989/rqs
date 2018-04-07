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
// atof() from common.c

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

/// A Quake-specific method of converting a string into a float.
///
/// Can convert:
///
/// * Integers in base 10 or base 16 (starts with 0x or 0X).
/// * Real numbers in base 10.
/// * A single-quoted ASCII character, using its underlying ASCII value.
/// * Any of these with a leading `-` sign (but **not** a leading `+` sign).
///
/// Unlike normal Rust, and like C, this function accepts input that is a
/// partially valid representation of a number. For example, `3.5foo` will be
/// converted to `3.5` --  the invalid trailing characters are discarded.
///
/// In the case where the string is completely invalid, `None` is returned.
/// This differs from the original implementation, which returned `0.0`.
pub fn atof(s: &str) -> Option<f32> {
    if s.len() == 0 {
        return None;
    }

    let mut i = 0;

    let sign = match &s[0..1] {
        "-" => {
            i += 1;
            -1.0
        },
        _ => 1.0,
    };
    let mut val = 0.0;

    // Permanently move past the minus sign, if it exists.
    let s = &s[i..];
    i = 0;

    // Try the hex case.
    if s.starts_with("0x") || s.starts_with("0X") {
        i += 2;
        let s = &s[i..];
        let mut got_some = false;

        for c in s.chars() {
            match c.to_digit(16) {
                Some(n) => {
                    got_some = true;
                    val = (val * 16.0) + (n as f32);
                },
                None => break,
            }
        }
        return match got_some {
            true => Some(sign * val),
            false => None,
        };
    }

    // Try the character case.
    if s.starts_with("'") {
        i += 1;
        let s = &s[i..];

        return match s.chars().next() {
            Some(c) if c.is_ascii() => Some(sign * (c as u32 as f32)),
            _ => None,
        };
    }

    // Try the decimal case.
    {
        let mut n_decimal = None;
        let mut n_total = 0;
        let mut got_some = false;
        let mut got_dot = false;

        for c in s.chars() {
            match c {
                '.' => match got_dot {
                    false => {
                        n_decimal = Some(n_total);
                        got_dot = true;
                    },
                    true => break,
                },
                _ => match c.to_digit(10) {
                    Some(n) => {
                        val = (val * 10.0) + (n as f32);
                        n_total += 1;
                        got_some = true;
                    },
                    None => break,
                },
            }
        }

        return match got_some {
            false => None,
            true => {
                match n_decimal {
                    None => Some(sign * val),
                    Some(d) => {
                        while n_total > d {
                            val = val / 10.0;
                            n_total -= 1;
                        }
                        Some(sign * val)
                    },
                }
            },
        }
    }
}


#[cfg(test)]
mod tests_cstr_buf {
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


#[cfg(test)]
mod tests_atof {
    use super::*;

    #[test]
    fn general_invalid() {
        assert_eq!(atof(""), None);
        assert_eq!(atof("+"), None);
        assert_eq!(atof("-"), None);
        assert_eq!(atof("!5"), None);
        assert_eq!(atof(" 5"), None);
        assert_eq!(atof("a5"), None);
    }

    #[test]
    fn hex_valid() {
        assert_eq!(atof("0x1"), Some(1.0));
        assert_eq!(atof("0x001"), Some(1.0));
        assert_eq!(atof("0x001"), Some(1.0));
        assert_eq!(atof("-0X123"), Some(-0x123 as f32));
        assert_eq!(atof("0x8f029b"), Some(0x8f029b as f32));
    }

    #[test]
    fn hex_partial_valid() {
        assert_eq!(atof("0x5g"), Some(5.0));
        assert_eq!(atof("0xf 1"), Some(0xf as f32));
        assert_eq!(atof("0xb.5"), Some(0xb as f32));
    }

    #[test]
    fn hex_invalid() {
        assert_eq!(atof("0x"), None);
        assert_eq!(atof("0X"), None);
        assert_eq!(atof("0xg"), None);
        assert_eq!(atof("0x 1"), None);
        assert_eq!(atof("+0x1"), None);
        assert_eq!(atof("-0x"), None);
    }

    #[test]
    fn char_valid() {
        // Stick to some printable characters.
        assert_eq!(atof("' "), Some(b' ' as f32));
        assert_eq!(atof("'!"), Some(b'!' as f32));
        assert_eq!(atof("'/"), Some(b'/' as f32));
        assert_eq!(atof("'0"), Some(b'0' as f32));
        assert_eq!(atof("'9"), Some(b'9' as f32));
        assert_eq!(atof("':"), Some(b':' as f32));
        assert_eq!(atof("'@"), Some(b'@' as f32));
        assert_eq!(atof("'A"), Some(b'A' as f32));
        assert_eq!(atof("'Z"), Some(b'Z' as f32));
        assert_eq!(atof("'a"), Some(b'a' as f32));
        assert_eq!(atof("'z"), Some(b'z' as f32));
        assert_eq!(atof("'{"), Some(b'{' as f32));
        assert_eq!(atof("'~"), Some(b'~' as f32));
        // The quote is also the char value.
        assert_eq!(atof("''"), Some(b'\'' as f32));
        // Closing quote isn't necessary, but should also work.
        assert_eq!(atof("'A'"), Some(b'A' as f32));
        // Too many quotes...
        assert_eq!(atof("''''''"), Some(b'\'' as f32));

        assert_eq!(atof("-'A"), Some(-(b'A' as i16) as f32));
    }

    #[test]
    fn char_partial_valid() {
        assert_eq!(atof("'! "), Some(b'!' as f32));
        assert_eq!(atof("'!potato"), Some(b'!' as f32));
        assert_eq!(atof("-'Aa"), Some(-(b'A' as i16) as f32));
    }

    #[test]
    fn char_invalid() {
        assert_eq!(atof("'"), None);
        assert_eq!(atof("+'a"), None);
        assert_eq!(atof("-'"), None);
        assert_eq!(atof("'💩"), None);
    }

    #[test]
    fn int_valid() {
        assert_eq!(atof("1"), Some(1.0));
        assert_eq!(atof("23"), Some(23.0));
        assert_eq!(atof("0081"), Some(81.0));
        assert_eq!(atof("-0081"), Some(-81.0));
    }

    #[test]
    fn int_partial_valid() {
        assert_eq!(atof("23d"), Some(23.0));
        assert_eq!(atof("23 1"), Some(23.0));
        assert_eq!(atof("23 .1"), Some(23.0));
        assert_eq!(atof("23-"), Some(23.0));
        assert_eq!(atof("23'1"), Some(23.0));
    }

    #[test]
    fn int_invalid() {
        assert_eq!(atof("+23"), None);
        assert_eq!(atof("[23"), None);
        assert_eq!(atof(" 23"), None);
    }

    #[test]
    fn float_valid() {
        assert_eq!(atof("1.0"), Some(1.0));
        assert_eq!(atof("23.0"), Some(23.0));
        assert_eq!(atof("23.9"), Some(23.9));
        assert_eq!(atof("0.7"), Some(0.7));
        assert_eq!(atof(".0"), Some(0.0));
        assert_eq!(atof(".7"), Some(0.7));
        assert_eq!(atof("-.7"), Some(-0.7));
        assert_eq!(atof("-9169241.26"), Some(-9169241.26));
    }

    #[test]
    fn float_partial_valid() {
        assert_eq!(atof("23."), Some(23.0));
        assert_eq!(atof("23..1"), Some(23.0));
        assert_eq!(atof("23.7d"), Some(23.7));
        assert_eq!(atof("23.7.5"), Some(23.7));
        assert_eq!(atof("-23.7.5"), Some(-23.7));
    }

    #[test]
    fn float_invalid() {
        assert_eq!(atof("..5"), None);
        assert_eq!(atof("-..5"), None);
        assert_eq!(atof("+5.0"), None);
    }
}
