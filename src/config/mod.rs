use std::fs::{metadata, Metadata};
use std::sync::{Arc, RwLock};
use std::path::{PathBuf, Path};

mod fingerprint;
mod http;
mod read;
mod root;
// sections
mod authorization;
mod authorizers;
mod handlers;
mod listen;
mod replication;
mod routing;
mod session_pools;
pub mod http_destinations;
pub mod ldap;
pub mod log;
pub mod networks;
// handlers
pub mod chat;
pub mod static_files;
pub mod proxy;
pub mod disk;
pub mod empty_gif;
pub mod redirect;
pub mod self_status;

pub use self::read::Error;
pub use self::root::ConfigData;
pub use self::listen::ListenSocket;
pub use self::handlers::Handler;
pub use self::authorizers::Authorizer;
pub use self::disk::Disk;
pub use self::empty_gif::EmptyGif;
pub use self::session_pools::{SessionPool};
pub use self::http::Destination;
pub use self::redirect::BaseRedirect;
pub use self::replication::Replication;

use quire::{parse_string, Options};

pub struct Configurator {
    path: PathBuf,
    file_metadata: Vec<(PathBuf, String, Metadata)>,
    cell: ConfigCell,
}

pub struct Config {
    data: ConfigData,
    fingerprint: fingerprint::Fingerprint,
}

#[derive(Clone)]
// TODO(tailhook) replace into ArcCell
pub struct ConfigCell(Arc<RwLock<Arc<Config>>>);

impl ::std::ops::Deref for Config {
    type Target = ConfigData;
    fn deref(&self) -> &ConfigData {
        &self.data
    }
}

impl ConfigCell {
    fn new(cfg: Config) -> ConfigCell {
        ConfigCell(Arc::new(RwLock::new(Arc::new(cfg))))
    }
    #[allow(dead_code)]
    pub fn from_string(data: &str, name: &str) -> Result<ConfigCell, Error> {
        let v = root::config_validator();
        let o = Options::default();
        Ok(ConfigCell::new(Config {
            data: parse_string(name, data, &v, &o)?,
            fingerprint: fingerprint::calc(&Vec::new())?,
        }))
    }
    pub fn get(&self) -> Arc<Config> {
        self.0.read()
            .expect("config exists")
            .clone()
    }
    pub fn fingerprint(&self) -> String {
        format!("{:x}", self.0.read()
            .expect("config cell is valid")
            .fingerprint)
    }
}

#[allow(dead_code)] // not used in main-dev
impl Configurator {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Configurator, Error> {
        let path = path.as_ref();
        let (cfg, meta) = read::read_config(path)?;
        Ok(Configurator {
            path: path.to_path_buf(),
            cell: ConfigCell::new(Config {
                data: cfg,
                fingerprint: fingerprint::calc(&meta)?,
            }),
            file_metadata: meta,
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
    #[allow(dead_code)]
    pub fn try_update(&mut self) -> Result<bool, Error> {
        let changed = self.file_metadata.iter()
            .any(|&(ref fname, _, ref old_meta)| {
                if let Ok(ref meta) = metadata(fname) {
                    fingerprint::compare_metadata(meta, old_meta)
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
        if **self.config().get() != new_cfg {
            let print = fingerprint::calc(&new_meta)?;
            self.file_metadata = new_meta;
            *self.cell.0.write()
                // we overwrite it so poisoned config is fine
                .unwrap_or_else(|p| p.into_inner())
                = Arc::new(Config {
                    data: new_cfg,
                    fingerprint: print,
                });
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
    use config::root::{ConfigData as Config, config_validator};

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
                listen: [127.0.0.1:2007]

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

    #[test]
    fn test_config() {
        let cfg = make_config();

        assert_eq!(cfg.listen.len(), 1);
        assert_eq!(cfg.routing.num_hosts(), 3+3);
        assert_eq!(cfg.handlers.len(), 7);
        assert_eq!(cfg.session_pools.len(), 1);
        assert_eq!(cfg.http_destinations.len(), 1);
        assert_eq!(cfg.disk_pools.len(), 0);

        assert_eq!(cfg.debug_routing, true);
        assert!(cfg.server_name.is_some());

        assert!(cfg.handlers.contains_key("example-chat"));
        assert!(cfg.handlers.contains_key("example-chat-http"));
        assert!(cfg.handlers.contains_key("empty-gif"));
        assert!(cfg.handlers.contains_key("websocket-echo-static"));
        assert!(cfg.handlers.contains_key("websocket-echo-html"));
        assert!(cfg.handlers.contains_key("websocket-echo"));
        assert!(cfg.handlers.contains_key("src"));

        assert!(cfg.session_pools.contains_key("example-session"));

        assert!(cfg.http_destinations.contains_key("superman"));
    }

    #[test]
    fn inactivity_timeouts() {
        use std::time::Duration;
        let cfg = make_config();

        let p = cfg.session_pools.get("example-session").unwrap();
        assert_eq!(*p.new_connection_idle_timeout, Duration::from_secs(60));
        assert_eq!(*p.client_min_idle_timeout, Duration::from_secs(1));
        assert_eq!(*p.client_max_idle_timeout, Duration::from_secs(7200));
        assert_eq!(*p.client_default_idle_timeout, Duration::from_secs(1));
    }
}
