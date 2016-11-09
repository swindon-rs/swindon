//! Chat protocol.
use std::str;
use rustc_serialize::json::{self, Json};

use futures::Future;
use tokio_core::reactor::{Handle};
use minihttp::enums::{Status, Method};
use minihttp::client::HttpClient;
use netbuf::Buf;

use websocket::{Dispatcher, Frame, ImmediateReplier, RemoteReplier, Error};
use websocket::Base64;
use super::message::{self, Message, MessageError};
use super::router::MessageRouter;


pub struct Chat {
    handle: Handle,
    client: HttpClient,
    router: MessageRouter,
    userinfo: Json,
    auth: String,
}

impl Chat {
    pub fn new(handle: Handle, client: HttpClient,
        router: MessageRouter, userinfo: Json)
        -> Chat
    {
        // TODO: userinfo is not needed here any more, only 'user_id'
        let auth = Chat::encode_userinfo(&userinfo);
        Chat {
            handle: handle,
            client: client,
            router: router,
            userinfo: userinfo,
            auth: auth,
        }
    }

    fn encode_userinfo(data: &Json) -> String {
        match data {
            &Json::Object(ref o) => {
                match o.get("user_id") {
                    Some(&Json::String(ref uid)) => {
                        let auth = format!("{{\"user_id\":{}}}",
                            json::encode(uid).unwrap());
                        format!("Tangle {}", Base64(auth.as_bytes()))
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Dispatcher for Chat {

    fn dispatch(&mut self, frame: Frame,
        replier: &mut ImmediateReplier, remote: &RemoteReplier)
        -> Result<(), Error>
    {
        let data = match frame {
            Frame::Text(data) => data,
            _ => return Ok(()),
        };

        match message::decode_message(data) {
            Ok((mut meta, msg)) => {
                let remote = remote.clone();

                let mut client = self.client.clone();
                let url = self.router.get_url(msg.method());
                let payload = msg.encode_with(&meta);
                client.request(Method::Post, url.as_str());
                client.add_header("Content-Type".into(), "application/json");
                client.add_header("Authorization".into(), self.auth.as_str());
                client.add_length(payload.as_bytes().len() as u64);
                // TODO: add Authorization header (with encoded user info);
                client.done_headers();
                client.write_body(payload.as_bytes());
                let call = client.done()
                    .map_err(|e| info!("Http Error: {:?}", e));

                self.handle.spawn(
                    call.and_then(move |resp| {
                        let result = parse_response(
                            resp.status, resp.body)
                            .map(|data| Message::Result(data))
                            .unwrap_or_else(|e| {
                                let e = Message::Error(e);
                                e.update_meta(&mut meta);
                                e
                            })
                            .encode_with(&meta);
                        remote.send_text(result.as_str())
                        .map_err(|e| info!("Remote send error: {:?}", e))
                    })
                );
            }
            Err(error) => {
                // TODO: make Error encodable
                let msg = Json::String(format!("{:?}", error));
                let msg = format!(
                    "[\"error\",{{\"error_kind\":\
                    \"validation_error\"}}, {}]", msg);
                replier.text(msg.as_str());
            }
        }
        Ok(())
    }
}


/// Parse backend response.
pub fn parse_response(status: Status, body: Option<Buf>)
    -> Result<Json, MessageError>
{
    // TODO: check content-type
    match status {
        Status::Ok => {
            let result = if let Some(ref body) = body {
                str::from_utf8(&body[..])
                .map_err(|e| MessageError::Utf8Error(e))
                .and_then(|s| Json::from_str(s)
                    .map_err(|e| MessageError::JsonError(e)))
            } else {
                Ok(Json::Null)
            };
            result
        }
        s => {
            let info = if let Some(ref body) = body {
                str::from_utf8(&body[..])
                .map_err(|e| MessageError::Utf8Error(e))
                .and_then(|s| Json::from_str(s)
                    .map_err(|e| MessageError::JsonError(e)))
                .ok()
            } else {
                None
            };
            Err(MessageError::HttpError(s, info))
        }
    }
}

#[cfg(test)]
mod test {

    use rustc_serialize::json::Json;
    use minihttp::Status;
    use netbuf::Buf;

    use super::parse_response;

    #[test]
    fn response_parsing() {
        let mut buf = Buf::new();
        buf.extend(b"[\"hello\",\"world\"]");
        let res = parse_response(Status::Ok, Some(buf)).unwrap();
        let res = res.as_array().unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], Json::String("hello".to_string()));
        assert_eq!(res[1], Json::String("world".to_string()));
    }
}
