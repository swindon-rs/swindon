use std::io::{self, Read};
use std::rc::Rc;
use std::fs::{File, Metadata, metadata};
use std::cell::RefCell;
use std::path::{PathBuf, Path, Component};

use quire::{self, Pos, Include, ErrorCollector, Options, parse_config};
use quire::{raw_parse as parse_yaml};
use quire::ast::{Ast, process as process_ast};

use super::Config;
use super::root::config_validator;
use super::Handler;


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



pub fn include_file(files: &RefCell<&mut Vec<(PathBuf, Metadata)>>,
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
                files.borrow_mut().push((path.to_path_buf(), metadata));
                parse_yaml(Rc::new(path.display().to_string()), &body,
                    |doc| { process_ast(&options, doc, err) },
                ).map_err(|e| err.add_error(e)).ok()
            })
            .unwrap_or_else(|| Ast::void(pos))
        }
    }
}

pub fn read_config<P: AsRef<Path>>(filename: P)
    -> Result<(Config, Vec<(PathBuf, Metadata)>), Error>
{
    let filename = filename.as_ref();
    let mut files = Vec::new();
    files.push((filename.to_path_buf(), metadata(filename)?));
    let cfg: Config = {
        let cell = RefCell::new(&mut files);
        let mut opt = Options::default();
        opt.allow_include(
            |a, b, c, d| include_file(&cell, a, b, c, d));
        parse_config(filename, &config_validator(), &opt)?
    };

    for (name, h) in &cfg.handlers {
        if let &Handler::SwindonChat(ref chat) = h {
            if !cfg.session_pools.contains_key(&chat.session_pool) {
                return Err(format!("No session pool {:?} defined",
                    chat.session_pool).into());
            }
        }
    }

    Ok((cfg, files))
}
