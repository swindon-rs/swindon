use std::sync::{Arc, RwLock};
use std::path::{PathBuf, Path};

mod read;
mod root;
mod http;
// sections
mod listen;
mod routing;
mod handlers;
// handlers
mod chat;
mod static_files;
mod proxy;

pub use self::read::Error;
pub use self::root::Config;
pub use self::listen::ListenSocket;


pub struct Configurator {
    path: PathBuf,
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
        let cfg = try!(read::read_config(path));
        Ok(Configurator {
            path: path.to_path_buf(),
            cell: ConfigCell(Arc::new(RwLock::new(Arc::new(cfg)))),
        })
    }
    pub fn config(&self) -> ConfigCell {
        self.cell.clone()
    }
    pub fn try_update(&self) -> Result<(), Error> {
        let new_cfg = try!(read::read_config(&self.path));
        *self.cell.0.write()
            // we overwrite it so poisoned config is fine
            .unwrap_or_else(|p| p.into_inner())
            = Arc::new(new_cfg);
        Ok(())
    }
}
