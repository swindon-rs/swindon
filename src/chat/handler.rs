//! Pull API handler.
use std::str;

use futures::{Async, Finished, finished};
use tokio_service::Service;
use tokio_core::net::TcpStream;
use tk_bufstream::IoBuf;
use minihttp::{Error, Request};
use minihttp::{ResponseFn, Status};
use minihttp::enums::Method;
use rustc_serialize::json::Json;

use {Pickler};
use config::ConfigCell;
use response::DebugInfo;
use default_error_page::write_error_page;

use super::ProcessorPool;

#[derive(Clone)]
pub struct ChatAPI {
    pub config: ConfigCell,
    pub chat_pool: ProcessorPool,
}


impl Service for ChatAPI {
    type Request = Request;
    type Response = ResponseFn<Finished<IoBuf<TcpStream>, Error>, TcpStream>;
    type Error = Error;
    type Future = Finished<Self::Response, Error>;

    fn call(&self, req: Request) -> Self::Future {
        let cfg = self.config.get();
        let status = match match_route(&req.method, req.path.as_str()) {
            Some(route) => {
                let payload = if let Some(ref body) = req.body {
                    str::from_utf8(&body.data[..]).ok()
                        .and_then(|s| Json::from_str(s).ok())
                } else {
                    None
                };
                if route.expect_data() && payload.is_some() {
                    // TODO: send message to processor;

                    //self.chat_processor.send_action()

                    Status::NoContent
                } else {
                    Status::BadRequest
                }
            }
            None => {
                Status::NotFound
            }
        };
        finished(ResponseFn::new(move |mut res| {
            let mut res = Pickler(res, cfg.clone(), DebugInfo::new(&req));
            write_error_page(status, res).done()
        }))
    }
}


fn match_route(method: &Method, path: &str) -> Option<ChatRoute> {
    use minihttp::enums::Method::*;

    if !path.starts_with("/v1/") {
        return None
    }
    let mut it = path.split("/").skip(2);   // skip '' & 'v1'
    let route = match (method, it.next()) {
        (&Post, Some(kind)) => {
            if !it.next().map(|x| x.len() > 0).unwrap_or(false) {
                return None
            }
            match kind {
                "lattice" => {
                    let (_, namespace) = path.split_at("/v1/lattice/".len());
                    ChatRoute::LatticeUpdate(namespace.replace("/", "."))
                }
                "topic" => {
                    let (_, topic) = path.split_at("/v1/topic/".len());
                    ChatRoute::TopicPublish(topic.replace("/", "."))
                }
                _ => return None
            }
        }
        (&Put, Some("connection")) |
        (&Delete, Some("connection")) => {
            let id = match it.next() {
                Some(id) if id.len() > 0 => id,
                _ => return None,
            };
            let kind = it.next();
            if !it.next().map(|x| x.len() > 0).unwrap_or(false) {
                return None
            }
            match kind {
                Some("subscriptions") => {
                    let (_, topic) = path.split_at(30 + id.len());
                    if method == &Put {
                        ChatRoute::TopicSubscribe(
                            id.to_string(), topic.replace("/", "."))
                    } else {
                        ChatRoute::TopicUnsubscribe(
                            id.to_string(), topic.replace("/", "."))
                    }
                }
                Some("lattices") => {
                    let (_, namespace) = path.split_at(25 + id.len());
                    if method == &Put {
                        ChatRoute::LatticeSubscribe(
                            id.to_string(), namespace.replace("/", "."))
                    } else {
                        ChatRoute::LatticeUnsubscribe(
                            id.to_string(), namespace.replace("/", "."))
                    }
                }
                _ => return None
            }
        }
        _ => return None
    };
    Some(route)
}


#[derive(Debug, PartialEq)]
pub enum ChatRoute {
    TopicSubscribe(String, String),
    TopicUnsubscribe(String, String),
    LatticeSubscribe(String, String),
    LatticeUnsubscribe(String, String),
    TopicPublish(String),
    LatticeUpdate(String),
}

impl ChatRoute {
    fn expect_data(&self) -> bool {
        use self::ChatRoute::*;
        match *self {
            TopicSubscribe(_, _) => false,
            TopicUnsubscribe(_, _) => false,
            LatticeSubscribe(_, _) => true,
            LatticeUnsubscribe(_, _) => false,
            TopicPublish(_) => true,
            LatticeUpdate(_) => true,
        }
    }
}


#[cfg(test)]
mod test {
    use minihttp::enums::Method;

    use super::match_route;
    use super::ChatRoute::*;

    #[test]
    fn match_topic_publish() {
        let path = "/v1/topic/test-chat/room1";
        let route = match_route(&Method::Post, path).unwrap();
        assert_eq!(route, TopicPublish("test-chat.room1".into()));
    }

    #[test]
    fn match_lattice_update() {
        let path = "/v1/lattice/test-chat/rooms";
        let route = match_route(&Method::Post, path).unwrap();
        assert_eq!(route, LatticeUpdate("test-chat.rooms".into()));
    }

    #[test]
    fn match_topic_subscribe() {
        let path = "/v1/connection/abcde/subscriptions/test-chat/room1";
        let route = match_route(&Method::Put, path).unwrap();
        assert_eq!(route, TopicSubscribe(
            "abcde".to_string(), "test-chat.room1".to_string()));
    }

    #[test]
    fn match_topic_unsubscribe() {
        let path = "/v1/connection/abcde/subscriptions/test-chat/room1";
        let route = match_route(&Method::Delete, path).unwrap();
        assert_eq!(route, TopicUnsubscribe(
            "abcde".to_string(), "test-chat.room1".to_string()));
    }

    #[test]
    fn match_lattice_subscribe() {
        let path = "/v1/connection/abcde/lattices/test-chat/room1";
        let route = match_route(&Method::Put, path).unwrap();
        assert_eq!(route, LatticeSubscribe(
            "abcde".to_string(), "test-chat.room1".to_string()));
    }

    #[test]
    fn match_lattice_unsubscribe() {
        let path = "/v1/connection/abcde/lattices/test-chat/room1";
        let route = match_route(&Method::Delete, path).unwrap();
        assert_eq!(route, LatticeUnsubscribe(
            "abcde".to_string(), "test-chat.room1".to_string()));
    }

    #[test]
    fn no_matches() {
        let path = "/v/";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Post, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Post, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/topic";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Post, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/topic/";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Post, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/topic/test-chat/room1";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/lattice/test-chat/rooms";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/connections//subscriptions/";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Post, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());

        let path = "/v1/connections/abc/subscriptions/";
        assert!(match_route(&Method::Get, path).is_none());
        assert!(match_route(&Method::Put, path).is_none());
        assert!(match_route(&Method::Post, path).is_none());
        assert!(match_route(&Method::Patch, path).is_none());
        assert!(match_route(&Method::Delete, path).is_none());
    }
}
