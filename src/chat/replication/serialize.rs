use std::str::FromStr;
use std::sync::Arc;
use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};
use rustc_serialize::json;

use intern::{SessionPoolName, Topic, Lattice as Namespace};
use chat::cid::{serialize_cid, Cid};
use chat::processor::Delta;
use super::{ReplAction, RemoteAction};

impl Encodable for ReplAction {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error>
    {
        if let &ReplAction::RemoteAction { ref pool, ref action } = self {
            s.emit_map(2, |s| {
                s.emit_map_elt_key(0, |s| s.emit_str("pool"))?;
                s.emit_map_elt_val(0, |s| pool.encode(s))?;
                s.emit_map_elt_key(1, |s| s.emit_str("action"))?;
                s.emit_map_elt_val(1, |s| action.encode(s))
            })
        } else {
            unreachable!()
        }
    }
}

impl Encodable for RemoteAction {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error>
    {
        use super::RemoteAction::*;
        s.emit_enum("RemoteAction", |s| {
            match *self {
                Subscribe { ref conn_id, ref topic } => {
                    s.emit_enum_struct_variant("Subscribe", 0, 2, |s| {
                        s.emit_enum_struct_variant_field("conn_id", 0, |s| {
                            s.emit_str(serialize_cid(&conn_id).as_str())
                        })?;
                        s.emit_enum_struct_variant_field("topic", 1, |s| {
                            topic.encode(s)
                        })
                    })
                }
                Unsubscribe { ref conn_id, ref topic } => {
                    s.emit_enum_struct_variant("Unsubscribe", 1, 2, |s| {
                        s.emit_enum_struct_variant_field("conn_id", 0, |s| {
                            s.emit_str(serialize_cid(&conn_id).as_str())
                        })?;
                        s.emit_enum_struct_variant_field("topic", 1, |s| {
                            topic.encode(s)
                        })
                    })
                }
                Publish { ref topic, ref data } => {
                    s.emit_enum_struct_variant("Publish", 2, 2, |s| {
                        s.emit_enum_struct_variant_field("topic", 0, |s| {
                            topic.encode(s)
                        })?;
                        let data = json::encode(data).expect("encodable"); // XXX:
                        s.emit_enum_struct_variant_field("data", 1, |s| {
                            s.emit_str(data.as_str())
                        })
                    })
                }
                Attach { ref conn_id, ref namespace } => {
                    s.emit_enum_struct_variant("Attach", 3, 2, |s| {
                        s.emit_enum_struct_variant_field("conn_id", 0, |s| {
                            s.emit_str(serialize_cid(&conn_id).as_str())
                        })?;
                        s.emit_enum_struct_variant_field("namespace", 1, |s| {
                            namespace.encode(s)
                        })
                    })
                }
                Detach { ref conn_id, ref namespace } => {
                    s.emit_enum_struct_variant("Detach", 4, 2, |s| {
                        s.emit_enum_struct_variant_field("conn_id", 0, |s| {
                            s.emit_str(serialize_cid(&conn_id).as_str())
                        })?;
                        s.emit_enum_struct_variant_field("namespace", 1, |s| {
                            namespace.encode(s)
                        })
                    })
                }
                Lattice { ref namespace, ref delta } => {
                    s.emit_enum_struct_variant("Lattice", 5, 2, |s| {
                        s.emit_enum_struct_variant_field("namespace", 0, |s| {
                            namespace.encode(s)
                        })?;
                        s.emit_enum_struct_variant_field("delta", 1, |s| {
                            delta.encode(s)
                        })
                    })
                }
            }
        })
    }
}

impl Decodable for ReplAction {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let (pool, action) = d.read_map(|d, size| {
            let mut pool = None;
            let mut action = None;
            for i in 0..size {
                match d.read_map_elt_key(i, |d| d.read_str())?.as_str() {
                    "pool" => {
                        pool = Some(d.read_map_elt_val(i, |d| {
                            SessionPoolName::decode(d)
                        })?);
                    }
                    "action" => {
                        action = Some(RemoteAction::decode(d)?);
                    }
                    k => {
                        return Err(d.error(format!(
                            "unexpected key: {}", k).as_str()))
                    }
                }
            };
            Ok((pool, action))
        })?;
        Ok(ReplAction::RemoteAction {
            pool: pool.ok_or(d.error("pool field is missing"))?,
            action: action.ok_or(d.error("action field is missing"))?,
        })
    }
}

const VARIANTS: [&'static str; 6] = [
    "Subscribe", "Unsubscribe", "Publish",
    "Attach", "Detach", "Lattice"];

impl Decodable for RemoteAction {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use super::RemoteAction::*;
        d.read_enum("RemoteAction", |d| {
            d.read_enum_variant(&VARIANTS[..], |d, idx| {
                let action = match idx {
                    0 | 1 => {
                        let cid = d.read_enum_struct_variant_field("conn_id", 0, |d| {
                            d.read_str().and_then(|s| Cid::from_str(s.as_str())
                            .map_err(|e| d.error(format!("{}", e).as_str())))
                        })?;
                        let topic = d.read_enum_struct_variant_field(
                            "topic", 1, |d| Topic::decode(d))?;
                        if idx == 0 {
                            Subscribe {
                                conn_id: cid,
                                topic: topic,
                            }
                        } else {
                            Unsubscribe {
                                conn_id: cid,
                                topic: topic,
                            }
                        }
                    }
                    2 => {
                        let topic = d.read_enum_struct_variant_field(
                            "topic", 0, |d| Topic::decode(d))?;
                        let data = d.read_enum_struct_variant_field(
                            "data", 1, |d| d.read_str()
                            .and_then(|s| json::Json::from_str(s.as_str())
                            .map_err(|e| d.error(format!("{}", e).as_str())))
                            )?;
                        Publish {
                            topic: topic,
                            data: Arc::new(data),
                        }
                    }
                    3 | 4 => {
                        let cid = d.read_enum_struct_variant_field("conn_id", 0, |d| {
                            d.read_str().and_then(|s| Cid::from_str(s.as_str())
                            .map_err(|e| d.error(format!("{}", e).as_str())))
                        })?;
                        let namespace = d.read_enum_struct_variant_field(
                            "namespace", 1, |d| Namespace::decode(d))?;
                        if idx == 0 {
                            Attach {
                                conn_id: cid,
                                namespace: namespace,
                            }
                        } else {
                            Detach {
                                conn_id: cid,
                                namespace: namespace,
                            }
                        }
                    }
                    5 => {
                        let namespace = d.read_enum_struct_variant_field(
                            "namespace", 0, |d| Namespace::decode(d))?;
                        let delta = d.read_enum_struct_variant_field(
                            "delta", 1, |d| Delta::decode(d))?;
                        Lattice {
                            namespace: namespace,
                            delta: delta,
                        }
                    }
                    _ => unreachable!()
                };
                Ok(action)
            })
        })
    }
}
