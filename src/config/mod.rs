use std::fs::{metadata, Metadata};
use std::sync::{Arc, RwLock};
use std::path::{PathBuf, Path};
use std::os::unix::fs::MetadataExt;

mod read;
mod root;
mod http;
// sections
mod listen;
mod routing;
mod handlers;
mod http_destinations;
// handlers
mod chat;
mod static_files;
mod proxy;

pub use self::read::Error;
pub use self::root::Config;
pub use self::listen::ListenSocket;


pub struct Configurator {
    path: PathBuf,
    file_metadata: Vec<(PathBuf, Metadata)>,
    cell: ConfigCell,
}


#[derive(Clone)]
pub struct ConfigCell(Arc<RwLock<Arc<Config>>>);

impl ConfigCell {
    pub fn get(self) -> Arc<Config> {
        self.0.read()
            .expect("config exists")
            .clone()
    }
}

impl Configurator {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Configurator, Error> {
        let path = path.as_ref();
        let (cfg, meta) = try!(read::read_config(path));
        Ok(Configurator {
            path: path.to_path_buf(),
            file_metadata: meta,
            cell: ConfigCell(Arc::new(RwLock::new(Arc::new(cfg)))),
        })
    }
    pub fn config(&self) -> ConfigCell {
        self.cell.clone()
    }
    /// Reread config
    ///
    /// Updates the reference to the config and returns Ok(true)
    /// if it's updated.
    ///
    /// If error occured old config is still active
    pub fn try_update(&mut self) -> Result<bool, Error> {
        let changed = self.file_metadata.iter()
            .any(|&(ref fname, ref oldmeta)| {
                if let Ok(meta) = metadata(fname) {
                    meta.modified().ok() != oldmeta.modified().ok() ||
                        meta.ino() != oldmeta.ino() ||
                        meta.dev() != oldmeta.dev()
                } else {
                    // We reread config on error for the case there is absent
                    // file that was previously present. And we want to account
                    // that
                    true
                }
            });
        if !changed {
            return Ok(false);
        }
        let (new_cfg, new_meta) = try!(read::read_config(&self.path));
        if *self.config().get() != new_cfg {
            self.file_metadata = new_meta;
            *self.cell.0.write()
                // we overwrite it so poisoned config is fine
                .unwrap_or_else(|p| p.into_inner())
                = Arc::new(new_cfg);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
