use std::io::{self, Read};
use std::rc::Rc;
use std::fs::{File, Metadata, metadata};
use std::cell::RefCell;
use std::path::{PathBuf, Path, Component};

use quire::{self, Pos, Include, ErrorCollector, Options, parse_config};
use quire::{raw_parse as parse_yaml};
use quire::ast::{Ast, process as process_ast};

use super::ConfigData;
use super::root::config_validator;
use super::Handler;
use config::static_files::Mode;
use config::log;
use intern::LogFormatName;


quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            display("IO error: {}", err)
            description("IO error")
            from()
        }
        Config(err: quire::ErrorList) {
            display("config error: {}", err)
            description("config error")
            from()
        }
        Validation(err: String) {
            display("validation error: {}", err)
            description("validation error")
            from()
        }
    }
}

macro_rules! err {
    // Shortcut to config post-load validation error
    ($msg:expr, $($a:expr),*) => (
        return Err(format!($msg, $($a),*).into())
    )
}


#[allow(dead_code)]
pub fn include_file(files: &RefCell<&mut Vec<(PathBuf, String, Metadata)>>,
    pos: &Pos, include: &Include,
    err: &ErrorCollector, options: &Options)
    -> Ast
{
    match *include {
        Include::File { filename } => {
            let mut path = PathBuf::from(&*pos.filename);
            path.pop(); // pop original filename
            for component in Path::new(filename).components() {
                match component {
                    Component::Normal(x) => path.push(x),
                    _ => {
                        // TODO(tailhook) should this error exist?
                        err.add_error(quire::Error::preprocess_error(pos,
                            format!("Only relative paths without parent \
                                     directories can be included")));
                        return Ast::void(pos);
                    }
                }
            }

            debug!("{} Including {:?}", pos, path);

            let mut body = String::new();
            File::open(&path)
            .and_then(|mut f| {
                let m = f.metadata();
                f.read_to_string(&mut body)?;
                m
            })
            .map_err(|e| {
                err.add_error(quire::Error::OpenError(path.clone(), e))
            }).ok()
            .and_then(|metadata| {
                files.borrow_mut().push((
                    path.to_path_buf(),
                    String::from(filename),
                    metadata,
                ));
                parse_yaml(Rc::new(path.display().to_string()), &body,
                    |doc| { process_ast(&options, doc, err) },
                ).map_err(|e| err.add_error(e)).ok()
            })
            .unwrap_or_else(|| Ast::void(pos))
        }
    }
}

pub fn read_config<P: AsRef<Path>>(filename: P)
    -> Result<(ConfigData, Vec<(PathBuf, String, Metadata)>), Error>
{
    let filename = filename.as_ref();
    let mut files = Vec::new();
    files.push((
        filename.to_path_buf(),
        String::from("<main>"),
        metadata(filename)?,
    ));
    let mut cfg: ConfigData = {
        let cell = RefCell::new(&mut files);
        let mut opt = Options::default();
        opt.allow_include(
            |a, b, c, d| include_file(&cell, a, b, c, d));
        parse_config(filename, &config_validator(), &opt)?
    };

    // Set some defaults
    if !cfg.log_formats.contains_key("debug-log") {
        cfg.log_formats.insert(LogFormatName::from("debug-log"),
            log::Format::from_string(r#"
                {{ request.client_ip }}
                {{ request.host }}
                "{{ request.method }}
                {{ request.path }}
                {{ request.version }}"
                {{ response.status_code }}
            "#.into()).expect("can always compile debug log"));
    }

    // Extra config validations

    for &(ref route, ref sub) in cfg.routing.hosts() {
        for (path, name) in sub {
            if cfg.handlers.get(name).is_none() {
                err!("Unknown handler for route: {:?} {:?}", route, name)
            }
            if path.as_ref().map(|x| x.ends_with("/")).unwrap_or(false) {
                err!("Path must not end with /: {:?} {:?} {:?}",
                     route, path, name);
            }
            if let Some(&Handler::StripWWWRedirect) = cfg.handlers.get(name) {
                if !route.matches_www() {
                    err!(concat!("Expected `www.` prefix for StripWWWRedirect",
                                 " handler route: {:?} {:?}"), route, name);
                }
            }
        }
    }
    for (name, h) in &cfg.handlers {
        match h {
            &Handler::SwindonChat(ref chat) => {
                match cfg.session_pools.get(&chat.session_pool) {
                    None => {
                        err!("No session pool {:?} defined", chat.session_pool)
                    }
                    Some(ref pool) => {
                        let dest = chat.message_handlers.resolve(
                            "tangle.session_inactive");
                        if !pool.inactivity_handlers.contains(dest) &&
                            pool.inactivity_handlers.len() != 0 {
                            err!(concat!(
                                "Inactivity destinations mismatch for",
                                 "{:?}: {:?}"), name, dest)
                        }
                    }
                }
                if let Some(h) = chat.http_route.as_ref() {
                    if !cfg.handlers.contains_key(h) {
                        err!("{:?}: unknown http route {:?}", name, h)
                    }
                }
                let u = &chat.message_handlers.default.upstream;
                if let Some(http_dest) = cfg.http_destinations.get(u) {
                    if http_dest.override_host_header.is_none() {
                        err!("http destination {:?} is used \
                             in message-handler of {:?}, so must contain \
                             override-host-header setting.", u, name);
                    }
                } else {
                    err!("{:?}: unknown http destination {:?}", name, u)
                }
                for (_, dest) in &chat.message_handlers.map {
                    if !cfg.http_destinations.contains_key(&dest.upstream) {
                        err!("{:?}: unknown http destination {:?}",
                             name, dest.upstream)
                    }
                }
            }
            &Handler::Proxy(ref proxy) => {
                let u = &proxy.destination.upstream;
                if !cfg.http_destinations.contains_key(u) {
                    err!("{:?}: unknown http destination {:?}", name, u)
                }
                if proxy.request_id_header.is_some() {
                    warn!(concat!(
                        "{:?}: request_id_header is deprecated",
                        " in !Proxy setting, it must be specified",
                        " in http destination"), name);
                }
            }
            &Handler::Static(ref config) => {
                if config.strip_host_suffix.is_some() &&
                   config.mode != Mode::with_hostname
                {
                    err!("{:?}: `strip-host-suffix` only \
                        works when `mode: with-hostname`", name);
                }
            }
            _ => {}
        }
    }
    // TODO: verify session_pool inactivity handlers
    for (name, s) in &cfg.session_pools {
        for dest in &s.inactivity_handlers {
            if !cfg.http_destinations.contains_key(&dest.upstream) {
                err!("{:?}: unknown http destination {:?}",
                     name, dest.upstream)
            }
        }
    }

    Ok((cfg, files))
}
