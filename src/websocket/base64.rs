// Part of this code is from rust-serialie, with the following copyright:
//
// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt::{self, Write};

static STANDARD_CHARS: &'static[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                        abcdefghijklmnopqrstuvwxyz\
                                        0123456789+/";

/// Format helper, to write base64 of the data
pub struct Base64<'a>(pub &'a [u8]);

impl<'a> fmt::Display for Base64<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // Deal with padding bytes
        let len = self.0.len();
        let mod_len = len % 3;

        let mut s_in = self.0[..len - mod_len].iter().map(|&x| x as u32);

        // Convenient shorthand
        let enc = |val| STANDARD_CHARS[val as usize];
        let mut write = |val| fmt.write_char(val as char).unwrap();

        // Iterate though blocks of 4
        while let (Some(first), Some(second), Some(third)) =
                    (s_in.next(), s_in.next(), s_in.next()) {

            let n = first << 16 | second << 8 | third;

            // This 24-bit number gets separated into four 6-bit numbers.
            write(enc((n >> 18) & 63));
            write(enc((n >> 12) & 63));
            write(enc((n >> 6 ) & 63));
            write(enc((n >> 0 ) & 63));
        }

        // Heh, would be cool if we knew this was exhaustive
        // (the dream of bounded integer types)
        match mod_len {
            0 => {},
            1 => {
                let n = (self.0[len-1] as u32) << 16;
                write(enc((n >> 18) & 63));
                write(enc((n >> 12) & 63));
                write(b'=');
                write(b'=');
            }
            2 => {
                let n = (self.0[len-2] as u32) << 16 |
                        (self.0[len-1] as u32) << 8;
                write(enc((n >> 18) & 63));
                write(enc((n >> 12) & 63));
                write(enc((n >> 6 ) & 63));
                write(b'=');
            }
            _ => panic!("Algebra is broken, please alert the math police")
        }
        Ok(())
    }
}
