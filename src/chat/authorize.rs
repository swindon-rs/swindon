use std::str::FromStr;
use std::sync::Arc;
use std::ascii::AsciiExt;
use std::borrow::Cow;

use futures::{AsyncSink};
use futures::sink::Sink;
use tk_http::Status;
use tk_http::server::Head;
use serde_json::{self, Value as Json};

use http_pools::{REQUESTS, FAILED_503};
use intern::SessionId;
use config::chat::Chat;
use incoming::{Input};
use chat::{Cid, MessageError, ConnectionSender};
use chat::MessageError::HttpError;
use chat::ConnectionMessage::FatalError;
use chat::backend;
use chat::message::AuthData;
use chat::processor::{Action};

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


pub fn start_authorize(inp: &Input, conn_id: Cid, settings: &Arc<Chat>,
                       messages: ConnectionSender)
{
    let pool = inp.runtime.session_pools
        .processor.pool(&settings.session_pool);
    pool.send(Action::NewConnection {
        conn_id: conn_id,
        channel: messages.clone(),
    });

    let dest = if settings.use_tangle_prefix() {
        settings.message_handlers.resolve("tangle.authorize_connection")
    } else {
        settings.message_handlers.resolve("swindon.authorize_connection")
    };

    let dest_settings = match inp.config.http_destinations.get(&dest.upstream)
    {
        Some(h) => h,
        None => {
            error!("No such destination {:?}", dest.upstream);
            messages.send(FatalError(
                HttpError(Status::InternalServerError, None)));
            return;
        }
    };

    let path: Cow<_> = if settings.use_tangle_prefix() {
        if dest.path == "/" {
            "/tangle/authorize_connection".into()
        } else {
            (dest.path.to_string() + "/tangle/authorize_connection").into()
        }
    } else {
        if dest.path == "/" {
            "/swindon/authorize_connection".into()
        } else {
            (dest.path.to_string() + "/swindon/authorize_connection").into()
        }
    };
    let mut up = inp.runtime.http_pools.upstream(&dest.upstream);

    let auth_data = match auth_data(inp.headers) {
        Ok(data) => data,
        Err(status) => {
            messages.send(FatalError(HttpError(status, None)));
            return;
        }
    };
    let codec = Box::new(backend::AuthCodec::new(path.into_owned(),
        conn_id, auth_data, pool.clone(),
        dest_settings,
        messages.clone(),
        inp.runtime.server_id.clone(),
        ));

    match up.get_mut().get_mut() {
        Some(pool) => {
            match pool.start_send(codec) {
                Ok(AsyncSink::NotReady(_codec)) => {
                    FAILED_503.incr(1);
                    messages.send(FatalError(
                        HttpError(Status::ServiceUnavailable, None)));
                }
                Ok(AsyncSink::Ready) => {
                    REQUESTS.incr(1);
                    debug!("Sent /swindon/authorize_connection to proxy");
                }
                Err(e) => {
                    error!("Error sending to pool {:?}: {}", dest.upstream, e);
                    // TODO(tailhook) is this situation possible?
                    //                probably this means this destination
                    //                removed from config, but we should
                    //                investigate it further
                    messages.send(FatalError(
                        HttpError(Status::ServiceUnavailable, None)));
                }
            }
        }
        None => {
            error!("No such destination {:?}", dest.upstream);
            messages.send(FatalError(HttpError(Status::NotFound, None)));
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
        Status::BadRequest|
        Status::Unauthorized|
        Status::Forbidden|
        Status::NotFound|
        Status::Gone|
        Status::InternalServerError|
        Status::ServiceUnavailable
        )
}

/// Parse userinfo received on Auth call;
pub fn parse_userinfo(data: &[u8])
    -> Result<(SessionId, Json), MessageError>
{
    use super::error::MessageError::ValidationError;

    let data: Json = serde_json::from_slice(data)?;
    let ssid = match data.get("user_id") {
        Some(&Json::String(ref ssid)) => {
            SessionId::from_str(ssid.as_str())
            .map_err(|_| ValidationError("Invalid user_id".into()))?
        }
        _ => {
            return Err(ValidationError(
                "user_id is missing or invalid".into()))
        }
    };
    Ok((ssid, data))
}
