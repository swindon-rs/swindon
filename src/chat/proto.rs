//! Chat protocol.
use std::io;
use std::collections::BTreeMap;
use rustc_serialize::json::{self, ToJson, Json};

use futures::{Future, Async, Poll};
use tokio_core::reactor::{Handle, Timeout};
use minihttp::enums::{Method, Header};
use minihttp::client::HttpClient;
use netbuf::Buf;

// use serde::{Serialize, Deserialize};
// use serde_json::{Value, Map};

use websocket::{Dispatcher, Frame, ImmediateReplier, RemoteReplier, Error};


pub struct Chat(pub Handle, pub HttpClient);

impl Dispatcher for Chat {

    fn dispatch(&mut self, frame: Frame,
        replier: &mut ImmediateReplier, remote: &RemoteReplier)
        -> Result<(), Error>
    {
        match frame {
            Frame::Text(data) => {
                if let Some(message) = decode(data) {
                    println!("Got message");
                    let remote = remote.clone();
                    let mut client = self.1.clone();
                    // TODO: make call to correct backend;
                    let payload = json::encode(&message).unwrap();
                    client.request(Method::Post,
                        format!("http://localhost:5000/{}", message.method())
                        .as_str());
                    client.add_header(
                        "Content-Type".into(), "application/json");
                    // client.add_header
                    client.add_length(payload.as_bytes().len() as u64);
                    client.done_headers();
                    client.write_body(payload.as_bytes());
                    let call = client.done()
                        .map_err(|e| info!("Http Error: {:?}", e));
                    self.0.spawn(call.and_then(move |resp| {
                            remote.send_text(
                                format!("Respnonse done: {:?}", resp).as_str())
                            .map_err(|e| info!("Remote send error: {:?}", e))
                        })
                    )
                };
            }
            _ => {}
        }
        Ok(())
    }
}

fn decode(data: &str) -> Option<Message> {
    let invalid_method = |m: &str| {
        m.starts_with("tangle.") | m.find("/").is_some()
    };
    match Json::from_str(data) {
        Ok(Json::Array(mut message)) => {
            if message.len() != 4 {
                trace!("Invalid args length: {}", message.len());
                return None
            }
            let kwargs = match message.pop() {
                Some(Json::Object(kwargs)) => kwargs,
                _ => {
                    trace!("kwargs not object");
                    return None
                }
            };
            let args = match message.pop() {
                Some(Json::Array(args)) => args,
                _ => {
                    trace!("args not array");
                    return None
                }
            };
            let meta = match message.pop() {
                Some(Json::Object(meta)) => {
                    if !meta.contains_key("request_id") {
                        trace!("meta missing 'request_id' key");
                        return None
                    }
                    meta
                }
                _ => {
                    trace!("meta is not object");
                    return None
                }
            };
            let method = match message.pop() {
                Some(Json::String(method)) => {
                    if invalid_method(&method) {
                        trace!("invalid method: {:?}", method);
                        return None
                    }
                    method
                }
                _ => {
                    trace!("method not string");
                    return None
                }
            };
            Some(Message(method, meta, args, kwargs))
        }
        _ => {
            trace!("message is not an array");
            return None
        }
    }
}

// #[derive(Debug, Serialize, Deserialize)]
// struct Msg(String, Map<String, Value>, Vec<Value>, Map<String, Value>);

#[derive(RustcEncodable)]
struct Message(String, BTreeMap<String, Json>,
    Vec<Json>, BTreeMap<String, Json>);

impl Message {
    pub fn method(&self) -> &str {
        self.0.as_str()
    }
}
