use std::io;
use std::path::Path;
use std::fs::read_dir;
use std::sync::Arc;

use tk_http::Status;
use trimmer::{Template, Context, Variable, Var, DataError};

use template;
use config::static_files::Static;

quick_error! {
    #[derive(Debug)]
    enum Error {
        TooManyFiles
        Io(err: io::Error) { from() }
    }
}

#[derive(Debug)]
struct Entry {
    name: String,
    is_dir: bool,
}

lazy_static! {
    static ref TEMPLATE: Template = template::PARSER.parse(
        include_str!("default_dir_index.html"))
        .expect("default dir index is a valid template");
}

fn read_files(path: &Path, settings: &Arc<Static>)
    -> Result<Vec<Entry>, Error>
{
    let mut result = Vec::new();
    for entry in read_dir(path)? {
        let entry = entry?;
        let typ = entry.file_type()?;
        result.push(Entry {
            name: Path::new(&entry.file_name()).display().to_string(),
            is_dir: typ.is_dir(),
        });
        if result.len() >= settings.generated_index_max_files {
            return Err(Error::TooManyFiles);
        }
    }
    Ok(result)
}

pub fn generate_index(path: &Path, virtual_path: &str,
    settings: &Arc<Static>)
    -> Result<Vec<u8>, Status>
{
    let files = match read_files(path, settings) {
        Ok(files) => files,
        Err(Error::TooManyFiles) => return Err(Status::Forbidden),
        Err(Error::Io(e)) => {
            error!("Error generating index for a directory {:?}: {}",
                path, e);
            return Err(Status::InternalServerError);
        }
    };
    let vpath = virtual_path.trim_right_matches('/');
    let mut ctx = Context::new();
    ctx.set("entries", &files);
    ctx.set("path", &vpath);
    let body = match TEMPLATE.render(&ctx) {
        Ok(body) => body,
        Err(e) => {
            error!("Error rendering directory index for {:?}: {}",
                path, e);
            return Err(Status::InternalServerError);
        }
    };
    Ok(body.into())
}

impl<'a> Variable<'a> for Entry {
    fn typename(&self) -> &'static str {
        "DirEntry"
    }
    fn attr<'x>(&'x self, attr: &str)
        -> Result<Var<'x, 'a>, DataError>
        where 'a: 'x
    {
        match attr {
            "name" => Ok(Var::borrow(&self.name)),
            "is_dir" => Ok(Var::borrow(&self.is_dir)),
            _ => Err(DataError::AttrNotFound),
        }
    }
}
