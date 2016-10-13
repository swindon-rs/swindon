use std::io::{self, Read};
use std::rc::Rc;
use std::fs::File;
use std::path::{PathBuf, Path, Component};

use quire::{self, Pos, Include, ErrorCollector, Options, parse_config};
use quire::parser::{parse as parse_yaml};
use quire::ast::{Ast, process as process_ast};

use super::Config;
use super::root::config_validator;


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
    }
}



pub fn include_file(pos: &Pos, include: &Include,
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
            .and_then(|mut f| f.read_to_string(&mut body))
            .map_err(|e| {
                err.add_error(quire::Error::OpenError(path.clone(), e))
            }).ok()
            .and_then(|_| {
                parse_yaml(Rc::new(path.display().to_string()), &body,
                    |doc| { process_ast(&options, doc, err) },
                ).map_err(|e| err.add_error(e)).ok()
            })
            .unwrap_or_else(|| Ast::void(pos))
        }
    }
}

pub fn read_config<P: AsRef<Path>>(filename: P) -> Result<Config, Error> {
    let mut opt = Options::default();
    opt.allow_include(include_file);
    let cfg = try!(parse_config(filename, &config_validator(), &opt));
    // TODO(tailhook) additional validations
    Ok(cfg)
}
