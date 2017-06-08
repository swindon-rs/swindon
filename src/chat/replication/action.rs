use std::sync::Arc;
use serde_json::Value as Json;

use runtime::RuntimeId;
use intern::{SessionPoolName, Topic, Lattice as Namespace};
use config::Replication;
use chat::Cid;
use chat::processor::{Action, Delta};
use super::OutgoingChannel;


#[derive(Debug)]
pub enum ReplAction {

    /// Attach new connection;
    Attach {
        tx: OutgoingChannel,
        peer: Option<String>,
        runtime_id: RuntimeId,
    },

    /// Send replicated message to remote peers;
    Outgoing(Message),

    /// Process message from remote peer;
    Incoming(Message),

    /// Reconnect known peers;
    Reconnect(Arc<Replication>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message(pub SessionPoolName, pub RemoteAction);


#[derive(Debug, Serialize, Deserialize)]
pub enum RemoteAction {
    Subscribe {
        conn_id: Cid,
        remote_id: RuntimeId,
        topic: Topic,
    },
    Unsubscribe {
        conn_id: Cid,
        remote_id: RuntimeId,
        topic: Topic,
    },
    Publish {
        topic: Topic,
        // TODO: probably use String here
        data: Arc<Json>,
    },

    Attach {
        conn_id: Cid,
        remote_id: RuntimeId,
        namespace: Namespace,
    },
    Detach {
        conn_id: Cid,
        remote_id: RuntimeId,
        namespace: Namespace,
    },
    Lattice {
        namespace: Namespace,
        // TODO: probably use String here
        delta: Delta,
    },
}

impl Into<Action> for RemoteAction {
    fn into(self) -> Action {
        use self::RemoteAction::*;
        match self {
            Subscribe { conn_id, topic, .. } => {
                Action::Subscribe {
                    conn_id: conn_id,
                    topic: topic,
                }
            }
            Unsubscribe { conn_id, topic, .. } => {
                Action::Unsubscribe {
                    conn_id: conn_id,
                    topic: topic,
                }
            }
            Publish { topic, data } => {
                Action::Publish {
                    topic: topic,
                    data: data,
                }
            }
            Attach { conn_id, namespace, .. } => {
                Action::Attach {
                    conn_id: conn_id,
                    namespace: namespace,
                }
            }
            Detach { conn_id, namespace, .. } => {
                Action::Detach {
                    conn_id: conn_id,
                    namespace: namespace,
                }
            }
            Lattice { namespace, delta } => {
                Action::Lattice {
                    namespace: namespace,
                    delta: delta,
                }
            }
        }
    }
}
