//! Chat protocol.
use std::str;
use rustc_serialize::json::Json;

use futures::Future;
use tokio_core::reactor::{Handle};
use minihttp::enums::{Status, Method};
use minihttp::client::HttpClient;
use netbuf::Buf;

use websocket::{Dispatcher, Frame, ImmediateReplier, RemoteReplier, Error};
use super::message::{Message, MessageError};
use super::router::MessageRouter;


pub struct Chat(pub Handle, pub HttpClient, pub MessageRouter);

impl Dispatcher for Chat {

    fn dispatch(&mut self, frame: Frame,
        replier: &mut ImmediateReplier, remote: &RemoteReplier)
        -> Result<(), Error>
    {
        let data = match frame {
            Frame::Text(data) => data,
            _ => return Ok(()),
        };

        match Message::decode(data) {
            Ok(message) => {
                let remote = remote.clone();

                let mut client = self.1.clone();
                // TODO: make call to correct backend;
                //      find proper route;
                //      resolve hostname to IP
                let url = self.2.get_url(message.method());
                let payload = message.payload();
                client.request(Method::Post, url.as_str());
                client.add_header("Content-Type".into(), "application/json");
                client.add_length(payload.as_bytes().len() as u64);
                client.done_headers();
                client.write_body(payload.as_bytes());
                let call = client.done()
                    .map_err(|e| info!("Http Error: {:?}", e));

                self.0.spawn(
                    call.and_then(move |resp| {
                        let result = parse_response(
                            resp.status, resp.body)
                            .map(|data| message.encode_result(data))
                            .unwrap_or_else(|e| message.encode_error(e));
                        remote.send_text(result.as_str())
                        .map_err(|e| info!("Remote send error: {:?}", e))
                    })
                );
            }
            Err(error) => {
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
