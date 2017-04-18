//! This should be globally unique identifier of request. But we also want it
//! to be k-ordered. I.e. it should be easy to sort request_ids by timestamp,
//! so we don't use UUID4.
//!
//! Structure:
//!
//! * 6 bytes timestamp in milliseconds (enough for 8k years)
//! * 12 (96 bits) bytes of random thread id
//! * 6 bytes per-thread sequential counter (wrapping)
//!
//! -> 32bytes base64-encoded id (24 bytes / 192 bits binary)
//!
//!
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{thread_rng, Rng};

scoped_thread_local!(static REQUEST_ID: RequestIdGenerator);

#[derive(Clone, Copy)]
pub struct RequestId([u8; 32]);

struct RequestIdGenerator {
    thread_id: [u8; 16],
    counter: AtomicUsize,
}

#[inline(always)]
fn base64triple(src: &[u8], dest: &mut [u8]) {
    // url-safe base64 chars
    const CHARS: &'static[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                  abcdefghijklmnopqrstuvwxyz\
                                  0123456789-_";
    debug_assert!(src.len() == 3);
    debug_assert!(dest.len() == 4);
    let n = ((src[0] as usize) << 16) |
            ((src[1] as usize) <<  8) |
             (src[2] as usize) ;
    dest[0] = CHARS[(n >> 18) & 63];
    dest[1] = CHARS[(n >> 12) & 63];
    dest[2] = CHARS[(n >>  6) & 63];
    dest[3] = CHARS[(n >>  0) & 63];
}

pub fn with_generator<F, R>(f: F) -> R
    where F: FnOnce() -> R
{
    let mut thread_id = [0u8; 12];
    thread_rng().fill_bytes(&mut thread_id);
    let mut thread_id_base64 = [0u8; 16];
    base64triple(&thread_id[..3], &mut thread_id_base64[..4]);
    base64triple(&thread_id[3..6], &mut thread_id_base64[4..8]);
    base64triple(&thread_id[6..9], &mut thread_id_base64[8..12]);
    base64triple(&thread_id[9..], &mut thread_id_base64[12..]);

    let gen = RequestIdGenerator {
        thread_id: thread_id_base64,
        counter: AtomicUsize::new(thread_rng().gen()),
    };
    REQUEST_ID.set(&gen, || {
        f()
    })
}

pub fn new() -> RequestId {
    let time = SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("system time is after epoch");
    let ms = (time.as_secs() * 1000 +
              (time.subsec_nanos() / 1000_000) as u64)
             & 0xFFFFFF_FFFFFF;  // ensure there are six bytes
    let mut buf = [0u8; 32];
    // Timestamp first
    base64triple(&[(ms >> 40) as u8, (ms >> 32) as u8, (ms >> 24) as u8],
                 &mut buf[0..4]);
    base64triple(&[(ms >> 16) as u8, (ms >>  8) as u8, (ms >>  0) as u8],
                 &mut buf[4..8]);
    let n = REQUEST_ID.with(|rid| {
        // Thread id second
        buf[8..24].copy_from_slice(&rid.thread_id);
        rid.counter.fetch_add(1, Ordering::SeqCst)
    });
    // Last is sequential number (only six bytes from it)
    base64triple(&[(n >> 40) as u8, (n >> 32) as u8, (n >> 24) as u8],
                 &mut buf[24..28]);
    base64triple(&[(n >> 16) as u8, (n >>  8) as u8, (n >>  0) as u8],
                 &mut buf[28..32]);
    return RequestId(buf);
}

impl RequestId {
    fn str(&self) -> &str {
        unsafe {
            str::from_utf8_unchecked(&self.0[..])
        }
    }

    pub fn from_str(val: &str) -> Option<RequestId> {
        if val.as_bytes().len() == 32 {
            let mut buf = [0u8; 32];
            buf[..].copy_from_slice(&val.as_bytes()[..32]);
            Some(RequestId(buf))
        } else {
            None
        }
    }
}

impl fmt::Debug for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rqid{:?}", self.str())
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.str().fmt(f)
    }
}
