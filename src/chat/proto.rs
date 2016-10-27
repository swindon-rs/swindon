//! Chat protocol.
use std::io;
use std::collections::BTreeMap;
use rustc_serialize::json::Json;

use futures::{Future, Async, Poll};
use tokio_core::reactor::{Handle, Timeout};
use netbuf::Buf;

// use serde::{Serialize, Deserialize};
// use serde_json::{Value, Map};

use websocket::{Dispatcher, Frame, ImmediateReplier, RemoteReplier, Error};


pub struct Chat(pub Handle);

impl Dispatcher for Chat {

    fn dispatch(&mut self, frame: Frame,
        replier: &mut ImmediateReplier, remote: &RemoteReplier)
        -> Result<(), Error>
    {
        // spawn call to backend with passthrough to ws;

        match frame {
            Frame::Text(data) => {
                if let Some(message) = decode(data) {
                    println!("Got message");
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
                return None
            }
            let kwargs = match message.pop() {
                Some(Json::Object(kwargs)) => kwargs,
                _ => return None
            };
            let args = match message.pop() {
                Some(Json::Array(args)) => args,
                _ => return None
            };
            let meta = match message.pop() {
                Some(Json::Object(meta)) => {
                    if !meta.contains_key("request_id") {
                        return None
                    }
                    meta
                }
                _ => return None
            };
            let method = match message.pop() {
                Some(Json::String(method)) => {
                    if invalid_method(&method) {
                        return None
                    }
                    method
                }
                _ => return None
            };
            Some(Message {
                method: method,
                meta: meta,
                args: args,
                kwargs: kwargs,
            })
        }
        _ => return None
    }
}

// #[derive(Debug, Serialize, Deserialize)]
// struct Msg(String, Map<String, Value>, Vec<Value>, Map<String, Value>);

struct Message {
    method: String,
    args: Vec<Json>,
    kwargs: BTreeMap<String, Json>,
    meta: BTreeMap<String, Json>,
}

struct ApiCall {
    msg: Message,
}

impl ApiCall {
}

impl Future for ApiCall
{
    type Item = Buf;
    type Error = io::Error;

    fn poll(&mut self)
        -> Poll<Self::Item, Self::Error>
    {
        // TODO: perform backend call:
        //  construct request
        //  (pick backend; setup path; setup headers; setup body);
        //  send request;
        //  read response;
        //  write response to websocket
        Ok(Async::NotReady)
    }
}


// service.call(Message) -> Future<Response, Error> -> poll() -> Ready(Response)
