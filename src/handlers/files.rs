use std::path::{Path, PathBuf, Component};
use std::sync::Arc;

use netbuf::Buf;
use futures::{BoxFuture, Future};
use minihttp::{Error};
use minihttp::request::Request;
use mime::TopLevel;
use mime_guess::guess_mime_type;
use tk_sendfile::DiskPool;
use tokio_core::net::TcpStream;
use futures_cpupool::CpuPool;

use config::static_files::{Static, Mode};
use {Pickler};


lazy_static! {
    // The amount of thread pools is important because it increases disk
    // parallelism and the ability for the kernel to merge disk requests
    static ref DISK_POOL: DiskPool = DiskPool::new(CpuPool::new(40));
}


pub fn serve(mut response: Pickler, path: PathBuf, settings: Arc<Static>)
    -> BoxFuture<(TcpStream, Buf), Error>
{
    // TODO(tailhook) check for symlink attacks
    let mime = guess_mime_type(&path);
    DISK_POOL.open(path)
    .and_then(move |file| {
        response.status(200, "OK");
        response.add_length(file.size());
        match (&mime.0, &settings.text_charset) {
            (&TopLevel::Text, &Some(ref enc)) => {
                response.format_header("Content-Type", format_args!(
                    "{}/{}; charset={}", mime.0, mime.1, enc));
            }
            _ => {
                response.format_header("Content-Type", mime);
            }
        }
        if response.done_headers() {
            response.steal_socket()
            .and_then(|(socket, buf)| {
                file.write_into(socket).map(|sock| (sock, buf))
            })
        } else {
            // Don't send any body
            unimplemented!();
        }
    }).map_err(|e| e.into()).boxed()
}

pub fn path(settings: &Static, suffix: &str, req: &Request)
    -> Result<PathBuf, ()>
{
    let path = match settings.mode {
        Mode::relative_to_site_root => &req.path,
        Mode::relative_to_route => suffix,
    };
    let path = Path::new(path.trim_left_matches('/'));
    let mut buf = settings.path.to_path_buf();
    for cmp in path.components() {
        match cmp {
            Component::Normal(chunk) => {
                buf.push(chunk);
            }
            _ => return Err(()),
        }
    }
    Ok(buf)
}
