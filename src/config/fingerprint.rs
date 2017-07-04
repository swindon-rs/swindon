use std::default::Default;
use std::io::{self, Write};
use std::fs::{File, Metadata};
use std::path::PathBuf;

use blake2::Blake2b;
use digest::FixedOutput;
use digest_writer::Writer;
use generic_array::GenericArray;
use typenum::U64;

pub type Fingerprint = GenericArray<u8, U64>;


#[cfg(unix)]
pub fn compare_metadata(meta: &Metadata, old_meta: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.modified().ok() != old_meta.modified().ok() ||
        meta.ino() != old_meta.ino() ||
        meta.dev() != old_meta.dev()
}

#[cfg(not(unix))]
pub fn compare_metadata(meta: &Metadata, old_meta: &Metadata) -> bool {
    meta.modified().ok() != old_meta.modified().ok()
}

pub fn calc(meta: &Vec<(PathBuf, String, Metadata)>)
    -> Result<Fingerprint, io::Error>
{
    let mut digest = Writer::new(Blake2b::default());
    for &(ref filename, ref name, ref meta) in meta {
        let mut file = File::open(filename)?;
        if compare_metadata(&file.metadata()?, meta) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        digest.write(name.as_bytes())?;
        digest.write(&[0])?;
        io::copy(&mut file, &mut digest)?;
    }
    Ok(digest.into_inner().fixed_result())
}
