use std::io;
use std::path::{PathBuf};
use std::sync::{Arc};
use std::str::from_utf8;

use tk_http::Status;
use http_file_headers::{Input as HeadersInput};

use config::static_files::{Static, Mode};
use default_error_page::{serve_error_page, error_page};
use incoming::{Input, Request, Transport};
use handlers::files::decode::decode_component;
use handlers::files::pools::get_pool;
use handlers::files::common::reply_file;


pub fn serve_dir<S: Transport>(settings: &Arc<Static>, mut inp: Input)
    -> Request<S>
{
    // TODO(tailhook) check for symlink attacks
    let path = match path(settings, &inp) {
        Ok(p) => p,
        Err(()) => {
            return serve_error_page(Status::Forbidden, inp);
        }
    };
    inp.debug.set_fs_path(&path);
    let pool = get_pool(&inp.runtime, &settings.pool);
    let settings = settings.clone();

    let hinp = HeadersInput::from_headers(&settings.headers_config,
        inp.headers.method(), inp.headers.headers());
    let fut = pool.spawn_fn(move || {
        hinp.probe_file(&path).map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                Status::Forbidden
            } else {
                error!("Error reading file {:?}: {}", path, e);
                Status::InternalServerError
            }
        })
    });

    reply_file(inp, pool, fut, move |e| {
        e.add_extra_headers(&settings.extra_headers);
    }, |e| {
        // TODO(tailhook) autoindex
        error_page(Status::Forbidden, e)
    })
}

pub fn path(settings: &Static, inp: &Input) -> Result<PathBuf, ()> {
    let path = match settings.mode {
        Mode::relative_to_domain_root | Mode::with_hostname => {
            inp.headers.path().unwrap_or("/")
        }
        Mode::relative_to_route => inp.suffix,
    };
    let path = match path.find(|c| c == '?' || c == '#') {
        Some(idx) => &path[..idx],
        None => path
    };
    let mut buf = Vec::with_capacity(path.len());
    if settings.mode == Mode::with_hostname {
        match inp.headers.host()  {
            Some(host) => {
                if host.find("/").is_some() {
                    // no slashes allowed
                    return Err(());
                }
                let name: &str = if let Some(colon) = host.find(":") {
                    &host[..colon]
                } else {
                    &host[..]
                };
                let name = if let Some(ref suf) = settings.strip_host_suffix {
                    if suf.len() >= name.len() {
                        // empty prefix is not allowed yet
                        return Err(());
                    }
                    if !name.ends_with(suf) {
                        // only this suffix should work
                        return Err(());
                    }
                    let final_dot = name.len() - suf.len() - 1;
                    if !name[final_dot..].starts_with('.') {
                        return Err(())
                    }
                    &name[..final_dot]
                } else {
                    name
                };
                buf.extend(name.as_bytes());
            }
            None => return Err(()),
        }
    }
    for cmp in path.split("/") {
        match cmp {
            "" | "." | "%2e" | "%2E" => {},
            ".." | "%2e." | "%2E." | ".%2e" | ".%2E"
            | "%2e%2e" | "%2E%2e" | "%2e%2E" | "%2E%2E" => return Err(()),
            _ => {
                if buf.len() > 0 {
                    buf.push(b'/');
                }
                decode_component(&mut buf, cmp)?;
            }
        }
    }

    // assert that we're not serving from root, this is a security check
    assert!(buf.len() == 0 || buf[0] != b'/');

    // only valid utf-8 supported so far
    let utf8 = from_utf8(&buf).map_err(|_| ())?;
    Ok(settings.path.join(utf8))
}
