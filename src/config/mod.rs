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
mod session_pools;
pub mod http_destinations;
// handlers
pub mod chat;
pub mod static_files;
pub mod proxy;
pub mod disk;
pub mod empty_gif;

pub use self::read::Error;
pub use self::root::Config;
pub use self::listen::ListenSocket;
pub use self::routing::Route;
pub use self::handlers::Handler;
pub use self::disk::Disk;
pub use self::empty_gif::EmptyGif;
pub use self::session_pools::SessionPool;
pub use self::http::Destination;

use quire::{parse_string, Options};

pub struct Configurator {
    path: PathBuf,
    file_metadata: Vec<(PathBuf, Metadata)>,
    cell: ConfigCell,
}


#[derive(Clone)]
pub struct ConfigCell(Arc<RwLock<Arc<Config>>>);

impl ConfigCell {
    fn new(cfg: Config) -> ConfigCell {
        ConfigCell(Arc::new(RwLock::new(Arc::new(cfg))))
    }
    pub fn from_string(data: &str, name: &str) -> Result<ConfigCell, Error> {
        let v = root::config_validator();
        let o = Options::default();
        Ok(ConfigCell::new(parse_string(name, data, &v, &o)?))
    }
    pub fn get(&self) -> Arc<Config> {
        self.0.read()
            .expect("config exists")
            .clone()
    }
}

impl Configurator {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Configurator, Error> {
        let path = path.as_ref();
        let (cfg, meta) = read::read_config(path)?;
        Ok(Configurator {
            path: path.to_path_buf(),
            file_metadata: meta,
            cell: ConfigCell::new(cfg),
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
        let (new_cfg, new_meta) = read::read_config(&self.path)?;
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

#[cfg(test)]
pub mod test {
    use std::sync::Arc;
    use quire::{parse_string, Options};
    use config::root::{Config, config_validator};

    pub fn make_config() -> Arc<Config> {
        let raw = r#"
            listen:
            - 127.0.0.1:8080

            debug-routing: true

            routing:
              localhost/empty.gif: empty-gif
              localhost/sources: src
              localhost/websocket.html: websocket-echo-static
              localhost/echo: websocket-echo-html
              localhost/websocket-echo: websocket-echo
              example.com: example-chat-http
              chat.example.com/: example-chat
              chat.example.com/css: example-chat-static
              chat.example.com/js: example-chat-static
              chat.example.com/index.html: example-chat-static

            handlers:

              example-chat: !SwindonChat

                session-pool: example-session
                http-route: example-chat-http

                message-handlers:
                  "*": superman/chat
                  sub.chat.*: superman/sub_chat
                  sub.chat: superman/sub
                  other.*: superman

              example-chat-http: !Proxy
                mode: forward
                ip-header: X-Remote-Ip
                destination: superman/

              empty-gif: !EmptyGif

              websocket-echo-static: !Static
                mode: relative_to_domain_root
                path: /work/public
                text-charset: utf-8

              websocket-echo-html: !SingleFile
                path: /work/public/websocket.html
                content-type: "text/html; charset=utf-8"

              websocket-echo: !WebsocketEcho

              src: !Static
                mode: relative_to_route
                path: /work/src
                text-charset: utf-8

            session-pools:
              example-session:
                listen: 127.0.0.1:2007

            http-destinations:
              superman:

                load-balancing: queue
                queue-size-for-503: 100k
                backend-connections-per-ip-port: 1
                in-flight-requests-per-backend-connection: 1

                addresses:
                - example.com:5000
        "#;
        let v = config_validator();
        let o = Options::default();
        let cfg: Config = parse_string("<inline>", raw, &v, &o).unwrap();
        Arc::new(cfg)
    }
}
