//! This code if from https://github.com/rust-lang/rust/pull/34724/files
//!
//! It's here until try_iter will be stable in rust
use std::sync::mpsc::Receiver;


/// An iterator that attempts to yield all pending values for a receiver.
/// `None` will be returned when there are no pending values remaining or
/// if the corresponding channel has hung up.
///
/// This Iterator will never block the caller in order to wait for data to
/// become available. Instead, it will return `None`.
pub struct TryIter<'a, T: 'a> {
    rx: &'a Receiver<T>
}

/// Returns an iterator that will attempt to yield all pending values.
/// It will return `None` if there are no more pending values or if the
/// channel has hung up. The iterator will never `panic!` or block the
/// user by waiting for values.
pub fn try_iter<T>(recv: &Receiver<T>) -> TryIter<T> {
    TryIter { rx: recv }
}

impl<'a, T> Iterator for TryIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> { self.rx.try_recv().ok() }
}
