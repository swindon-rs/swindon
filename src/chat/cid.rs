use std::str::FromStr;
use std::num::ParseIntError;

/// Internal connection id
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Cid(u64);


impl Cid {
    #[cfg(target_pointer_width = "64")]
    pub fn new() -> Cid {
        // Until atomic u64 really works
        use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
        static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        Cid(COUNTER.fetch_add(1, Ordering::Relaxed) as u64)
    }
}

// TODO: make these two functions properly serialize and deserialize Cid;
pub fn serialize_cid(cid: &Cid) -> String {
    format!("{}", cid.0)
}

impl FromStr for Cid {
    type Err = ParseIntError;

    fn from_str(src: &str) -> Result<Cid, Self::Err> {
        src.parse().map(|x| Cid(x))
    }
}
