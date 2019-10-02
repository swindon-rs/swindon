//! This is a thread that processes websocket messages
//!
//! Important things about this thread:
//!
//! 1. It's fully driven by input message
//! 2. No timers here
//! 3. No time management here, only accept time value from events (*)
//!
//! This allows much better unit tests

use std::time::Instant;
use std::sync::Arc;
use std::collections::HashMap;
use std::fmt;

use serde_json::Value as Json;
use serde::ser::{Serialize, Serializer, SerializeTuple};
use futures::sync::mpsc::{UnboundedSender as Sender};

use crate::config;
use crate::intern::{Topic, SessionId, SessionPoolName, Lattice as Namespace};
use crate::intern::{LatticeKey, LatticeVar};
use crate::chat::{Cid, ConnectionSender, CloseReason};
use crate::chat::message::{Meta, MetaWithExtra};
use crate::chat::error::MessageError;

mod main;
mod pool;
mod pair;
mod public;
mod session;
mod heap;
mod try_iter;  // temporary
mod connection;
mod lattice;

pub use self::public::{Processor, ProcessorPool};
pub use self::lattice::Delta;
pub use self::main::{SESSION_POOLS};
pub use self::pool::{ACTIVE_SESSIONS, INACTIVE_SESSIONS};
pub use self::pool::{PUBSUB_INPUT, PUBSUB_OUTPUT, TOPICS};
pub use self::pool::{LATTICES};
pub use self::lattice::{SHARED_KEYS, PRIVATE_KEYS};
pub use self::lattice::{SHARED_COUNTERS, PRIVATE_COUNTERS};
pub use self::lattice::{SHARED_SETS, PRIVATE_SETS};
pub use self::lattice::{SHARED_REGISTERS, PRIVATE_REGISTERS};
pub use self::lattice::{SET_ITEMS};

lazy_static! {
    pub static ref SWINDON_USER: Namespace = Namespace::from("swindon.user");
    pub static ref STATUS_VAR: LatticeVar = LatticeVar::from("status");
    pub static ref ACTIVE_STATUS: Arc<Json> = Arc::new(Json::from("active"));
    pub static ref INACTIVE_STATUS: Arc<Json> =
        Arc::new(Json::from("inactive"));
    pub static ref OFFLINE_STATUS: Arc<Json> =
        Arc::new(Json::from("offline"));
}

#[derive(Debug)]
pub struct Event {
    pool: SessionPoolName,
    timestamp: Instant,
    action: Action,
}

// TODO(tailhook) move it upper the stack (to chat::)
#[derive(Debug)]
pub enum ConnectionMessage {
    /// Topic publish message:
    /// `["message", {"topic": topic}, data]`
    Publish(Topic, Arc<Json>),
    /// Auth response message:
    /// `["hello", {}, json_data]`
    ///
    /// Note: SessionId here is not serialized, and goes only to dispatcher
    Hello(SessionId, Arc<Json>),
    /// Websocket call result;
    Result(Arc<Meta>, Json),
    /// Lattice update message
    Lattice(Namespace, Arc<HashMap<LatticeKey, lattice::Values>>),
    /// Error response to websocket call
    Error(Arc<Meta>, MessageError),
    /// Force websocket stop by backend
    FatalError(MessageError),
    /// Just stop the socket probably by to close message
    StopSock(CloseReason),
}

#[derive(Debug)]
pub enum PoolMessage {
    InactiveSession {
        session_id: SessionId,
        // This is mostly for debugging for now
        connections_active: usize,
        metadata: Arc<Json>,
    },
}

pub enum Action {

    // ------ Session pool management ------
    //   For all actions session pool name is passed in event structure
    NewSessionPool {
        config: Arc<config::SessionPool>,
        channel: Sender<PoolMessage>,
    },
    StopSessionPool,

    // ------ Connection management ------
    NewConnection {
        conn_id: Cid,
        channel: ConnectionSender,
    },
    Associate {
        conn_id: Cid,
        session_id: SessionId,
        metadata: Arc<Json>
    },
    UpdateActivity {
        session_id: SessionId,
        // We receive duration from client, but we expect request handling
        // code to validate and normalize it for us
        timestamp: Instant,
    },
    Disconnect {
        conn_id: Cid,
    },

    // ------ Subscriptions ------
    Subscribe {
        conn_id: Cid,
        topic: Topic,
    },
    Unsubscribe {
        conn_id: Cid,
        topic: Topic,
    },
    Publish {
        topic: Topic,
        data: Arc<Json>,
    },

    // ------ Lattices ------
    /// Attaches (subscribes to) lattice for this user
    ///
    /// Sends current lattice data to this connection immediately
    ///
    /// Note: data must *already* be in there
    Attach {
        namespace: Namespace,
        conn_id: Cid,
    },
    /// Updates data in lattice
    ///
    /// This works both for initial attach (subscription) of lattice and
    /// for subsequent updates
    ///
    /// Note: this message must be sent *before* Attach when connection
    /// initially attaches to the lattice
    Lattice {
        namespace: Namespace,
        delta: Delta,
    },
    Detach {
        namespace: Namespace,
        conn_id: Cid,
    },
    AttachUsers {
        conn_id: Cid,
        list: Vec<SessionId>,
    },
    UpdateUsers {
        session_id: SessionId,
        list: Vec<SessionId>,
    },
    DetachUsers {
        conn_id: Cid,
    },
}

// TODO(tailhook) optimize to not to create serde_json::Value
pub fn json_err(err: &MessageError) -> Json {
    match err {
        &MessageError::HttpError(ref status, _) => {
            json!({
                "error_kind": "http_error",
                "http_error": status.code(),
            })
        }
        &MessageError::Utf8Error(_) => {
            json!({"error_kind": "data_error"})
        }
        &MessageError::JsonError(_) => {
            json!({"error_kind": "data_error"})
        }
        &MessageError::ValidationError(_) => {
            json!({"error_kind": "validation_error"})
        }
        _ => {
            json!({"error_kind": "internal_error"})
        }
    }
}

impl Serialize for ConnectionMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        use self::ConnectionMessage::*;
        let mut tup = serializer.serialize_tuple(3)?;
        match *self {
            Publish(ref topic, ref json) => {
                #[derive(Serialize)]
                struct Meta<'a> {
                    topic: &'a Topic,
                }
                tup.serialize_element("message")?;
                tup.serialize_element(&Meta { topic: topic })?;
                tup.serialize_element(json)?;
            }
            // We don't serialize session id, it's already in dict
            Hello(_, ref json) => {
                tup.serialize_element("hello")?;
                tup.serialize_element(&json!({}))?;
                tup.serialize_element(json)?;
            }
            Lattice(ref namespace, ref json) => {
                #[derive(Serialize)]
                struct Meta<'a> {
                    namespace: &'a Namespace,
                }
                tup.serialize_element("lattice")?;
                tup.serialize_element(&Meta { namespace: namespace })?;
                tup.serialize_element(json)?;
            }
            Result(ref meta, ref json) => {
                tup.serialize_element("result")?;
                tup.serialize_element(&meta)?;
                tup.serialize_element(json)?;
            }
            Error(ref meta, ref err) => {
                tup.serialize_element("error")?;
                tup.serialize_element(&MetaWithExtra {
                    meta: meta, extra: json_err(err),
                })?;
                tup.serialize_element(&err)?;
            }
            FatalError(ref err) => {
                // this clause should never actually be called
                // but we think it's unwise to put assertions in serializer
                tup.serialize_element("fatal_error")?;
                tup.serialize_element(&json_err(err))?;
                tup.serialize_element(&json!(null))?;
            }
            StopSock(ref reason) => {
                // this clause should never actually be called
                // but we think it's unwise to put assertions in serializer
                tup.serialize_element("stop")?;
                tup.serialize_element(&format!("{:?}", reason))?;
                tup.serialize_element(&json!(null))?;
            }
        }
        tup.end()
    }
}

// NOTE: UnboundSender does not derive from Debug.
impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Action::*;
        match self {
            &NewSessionPool {..} => {
                write!(f, "Action::NewSessionPool")
            }
            &StopSessionPool => {
                write!(f, "Action::StopSessionPool")
            }
            &NewConnection { ref conn_id, .. } => {
                write!(f, "Action::NewConnection({:?})", conn_id)
            }
            &Associate { ref conn_id, ref session_id, .. } => {
                write!(f, "Action::Associate({:?}, {:?})", conn_id, session_id)
            }
            &UpdateActivity { ref session_id, .. } => {
                write!(f, "Action::UpdateActivity({:?})", session_id)
            }
            &Disconnect { ref conn_id } => {
                write!(f, "Action::Disconnect({:?})", conn_id)
            }
            &Subscribe { ref conn_id, ref topic } => {
                write!(f, "Action::Subscribe({:?}, {:?})", conn_id, topic)
            }
            &Unsubscribe { ref conn_id, ref topic } => {
                write!(f, "Action::Unsubscribe({:?}, {:?})", conn_id, topic)
            }
            &Publish { ref topic, .. } => {
                write!(f, "Action::Publish({:?})", topic)
            }
            &Attach { ref conn_id, ref namespace } => {
                write!(f, "Action::Attach({:?}, {:?})", conn_id, namespace)
            }
            &Lattice { ref namespace, .. } => {
                write!(f, "Action::Lattice({:?})", namespace)
            }
            &Detach { ref conn_id, ref namespace } => {
                write!(f, "Action::Detach({:?}, {:?})", conn_id, namespace)
            }
            &AttachUsers { ref conn_id, ref list } => {
                write!(f, "Action::AttachUsers({:?}, {})",
                    conn_id, list.len())
            }
            &UpdateUsers { ref session_id, ref list } => {
                write!(f, "Action::UpdateUsers({:?}, {})",
                    session_id, list.len())
            }
            &DetachUsers { ref conn_id } => {
                write!(f, "Action::Detach({:?})", conn_id)
            }
        }
    }
}
