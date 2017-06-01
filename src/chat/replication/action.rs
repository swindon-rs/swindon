use std::net::SocketAddr;
use std::sync::Arc;
use std::fmt;
use serde_json::Value as Json;
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};

use runtime::RuntimeId;
use intern::{SessionPoolName, Topic, Lattice as Namespace};
use chat::Cid;
use chat::processor::{Action, Delta};
use super::OutgoingChannel;


#[derive(Debug)]
pub enum ReplAction {

    Attach {
        tx: OutgoingChannel,
        peer: String,
        addr: SocketAddr,
        runtime_id: RuntimeId,
    },

    RemoteAction {
        pool: SessionPoolName,
        action: RemoteAction,
    },
}


#[derive(Debug, Serialize, Deserialize)]
pub enum RemoteAction {
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
        // TODO: probably use String here
        data: Arc<Json>,
    },

    Attach {
        conn_id: Cid,
        namespace: Namespace,
    },
    Detach {
        conn_id: Cid,
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
            Subscribe { conn_id, topic } => {
                Action::Subscribe {
                    conn_id: conn_id,
                    topic: topic,
                }
            }
            Unsubscribe { conn_id, topic } => {
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
            Attach { conn_id, namespace } => {
                Action::Attach {
                    conn_id: conn_id,
                    namespace: namespace,
                }
            }
            Detach { conn_id, namespace } => {
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

impl Serialize for ReplAction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        if let &ReplAction::RemoteAction { ref pool, ref action } = self {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry(&"pool", pool)?;
            map.serialize_entry(&"action", action)?;
            map.end()
        } else {
            unreachable!()
        }
    }
}

struct ReplVisitor;
impl<'de> Visitor<'de> for ReplVisitor {
    type Value = ReplAction;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result
    {
        formatter.write_str("mapping describing ReplAction")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where M: MapAccess<'de>
    {
        let mut pool = None;
        let mut action = None;
        while let Some(key) = access.next_key::<&str>()? {
            match key {
                "pool" => {
                    pool = Some(access.next_value()?);
                }
                "action" => {
                    action = Some(access.next_value()?);
                }
                _ => return Err(de::Error::custom("unexpected key"))
            }
        }
        if let (Some(p), Some(a)) = (pool, action) {
            Ok(ReplAction::RemoteAction { pool: p, action: a })
        } else {
            Err(de::Error::custom("invalid action"))
        }
    }
}

impl<'de> Deserialize<'de> for ReplAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_map(ReplVisitor)
    }
}
