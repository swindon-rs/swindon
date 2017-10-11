use std::sync::Arc;
use std::time::{Instant, Duration};
use serde_json::Value as Json;

use runtime::ServerId;
use intern::{SessionId, SessionPoolName, Topic, Lattice as Namespace};
use config::Replication;
use chat::{Cid, SyncData};
use chat::processor::{Action, Delta};
use super::OutgoingChannel;


#[derive(Debug)]
pub enum ReplAction {

    /// Attach new connection;
    Attach {
        tx: OutgoingChannel,
        peer: Option<String>,
        server_id: ServerId,
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
    InitialSync {
        server_id: ServerId,
        state: SyncData,
    },
    Subscribe {
        conn_id: Cid,
        server_id: ServerId,
        topic: Topic,
    },
    Unsubscribe {
        conn_id: Cid,
        server_id: ServerId,
        topic: Topic,
    },
    Publish {
        topic: Topic,
        // TODO: probably use String here
        data: Arc<Json>,
    },

    Attach {
        conn_id: Cid,
        server_id: ServerId,
        namespace: Namespace,
    },
    Detach {
        conn_id: Cid,
        server_id: ServerId,
        namespace: Namespace,
    },
    Lattice {
        namespace: Namespace,
        // TODO: probably use String here
        delta: Delta,
    },
    AttachUsers {
        conn_id: Cid,
        server_id: ServerId,
        list: Vec<SessionId>,
    },
    UpdateUsers {
        session_id: SessionId,
        list: Vec<SessionId>,
    },
    DetachUsers {
        conn_id: Cid,
        server_id: ServerId,
    },

    // NOTE: In remote action we send original duration, not timestamp;
    UpdateActivity {
        session_id: SessionId,
        duration: Duration,
    },
}

impl Into<Action> for RemoteAction {
    fn into(self) -> Action {
        use self::RemoteAction::*;
        // server_id is ignored here, because it's filtered
        // in Watcher::local_send
        match self {
            InitialSync { server_id, state } => {
                Action::PeerSync { server_id, state }
            }
            Subscribe { conn_id, topic, server_id: _ } => {
                Action::Subscribe {
                    conn_id: conn_id,
                    topic: topic,
                }
            }
            Unsubscribe { conn_id, topic, server_id: _ } => {
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
            Attach { conn_id, namespace, server_id: _ } => {
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
            UpdateActivity { session_id, duration } => {
                Action::UpdateActivity {
                    session_id: session_id,
                    timestamp: Instant::now() + duration,
                }
            }
            AttachUsers { conn_id, list, server_id: _ } => {
                Action::AttachUsers {
                    conn_id: conn_id,
                    list: list,
                }
            }
            UpdateUsers { session_id, list } => {
                Action::UpdateUsers { session_id, list }
            }
            DetachUsers { conn_id, server_id: _ } => {
                Action::DetachUsers {
                    conn_id: conn_id,
                }
            }
        }
    }
}
