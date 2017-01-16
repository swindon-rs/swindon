use std::str::FromStr;
use std::sync::Arc;
use std::ascii::AsciiExt;
use std::borrow::Cow;

use futures::{Future, AsyncSink};
use futures::sink::Sink;
use futures::sync::oneshot::{Sender};
use futures::sync::mpsc::{unbounded as channel};
use minihttp::Status;
use minihttp::server::Head;
use rustc_serialize::json::Json;

use intern::SessionId;
use config::chat::Chat;
use incoming::{Input};
use chat::{Cid, MessageError};
use chat::backend;
use chat::message::AuthData;
use chat::processor::{Action, ConnectionMessage};

/// Issue Auth call to backend.
///
/// Send Auth message to proper backend
/// returninng Hello/Error message.
fn auth_data(handshake: &Head) -> Result<AuthData, Status> {
    let mut cookie = None;
    let mut auth = None;
    for (key, value) in handshake.headers() {
        if key.eq_ignore_ascii_case("Cookie") {
            if cookie.is_some() {
                debug!("Duplicate Cookie header");
                return Err(Status::BadRequest);
            }
            cookie = Some(String::from_utf8_lossy(value).into_owned());
        } else if key.eq_ignore_ascii_case("Authorization") {
            if auth.is_some() {
                debug!("Duplicate Authorization header");
                return Err(Status::BadRequest);
            }
            auth = Some(String::from_utf8_lossy(value).into_owned());
        }
    }
    let url_qs = handshake.path().expect("invalid path for websocket hanshake")
        .splitn(2, "?").nth(1).unwrap_or("").to_string();

    Ok(AuthData {
        http_cookie: cookie,
        http_authorization: auth,
        url_querystring: url_qs,
    })
}


pub fn start_authorize(inp: &Input, settings: &Arc<Chat>,
                       response: Sender<Result<(SessionId, Json), Status>>)
{
    let conn_id = Cid::new();
    let (tx, rx) = channel();

    let pool = inp.runtime.chat_processor.read()
        .expect("chat pool locked")
        .pool(&settings.session_pool);
    let sess_cfg = inp.config.session_pools
        .get(&settings.session_pool).unwrap(); // FIXME: unwrap

    pool.send(Action::NewConnection {
        conn_id: conn_id,
        channel: tx,
    });

    let dest = settings.message_handlers
        .resolve("tangle.authorize_connection");
    let path: Cow<_> = if dest.path == "/" {
        "/tangle/authorize_connection".into()
    } else {
        (dest.path.to_string() + "/tangle/authorize_connection").into()
    };
    let mut up = inp.runtime.http_pools.upstream(&dest.upstream);

    let auth_data = match auth_data(inp.headers) {
        Ok(data) => data,
        Err(e) => {
            response.complete(Err(e));
            return;
        }
    };
    let codec = Box::new(backend::AuthCodec::new(path.into_owned(),
        conn_id, auth_data, response));

    match up.get_mut().get_mut() {
        Some(pool) => {
            match pool.start_send(codec) {
                Ok(AsyncSink::NotReady(codec)) => {
                    // codec.into_inner().send(Err(Status::ServiceUnavailable))
                    unimplemented!();
                }
                Ok(AsyncSink::Ready) => {
                    debug!("Sent /tangle/authorize_connection to proxy");
                }
                Err(e) => {
                    error!("Error sending to pool {:?}: {}", dest.upstream, e);
                    // TODO(tailhook) ensure that sender is closed
                }
            }
        }
        None => {
            error!("No such pool {:?}", dest.upstream);
            // codec.into_inner().send(Err(Status::NotFound))
            unimplemented!();
        }
    }

}

/// Returns true when status is one in the set which backend is allowed
/// (and expected) to return
///
/// All http statuses returned from a backend that doesn't match this list
/// will be logged.
pub fn good_status(status: Status) -> bool {
    matches!(status,
        Status::Forbidden|
        Status::Unauthorized|
        Status::NotFound|
        Status::Gone|
        Status::BadRequest)
}

/// Parse userinfo received on Auth call;
pub fn parse_userinfo(response: Json)
    -> Result<(SessionId, Json), MessageError>
{
    use super::message::ValidationError::*;
    use super::error::MessageError::*;
    match response {
        Json::Object(data) => {
            let sess_id = match data.get("user_id".into()) {
                Some(&Json::String(ref s)) => {
                    SessionId::from_str(s.as_str())
                    .map_err(|_| ValidationError(InvalidUserId))?
                }
                _ => return Err(ValidationError(InvalidUserId)),
            };
            Ok((sess_id, Json::Object(data)))
        }
        _ => {
            Err(ValidationError(ObjectExpected))
        }
    }
}
