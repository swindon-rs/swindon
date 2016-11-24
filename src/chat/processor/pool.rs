use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::{HashMap, HashSet};

use rustc_serialize::json::Json;
use futures::sync::mpsc::{UnboundedSender as Sender};

use intern::{Topic, SessionId, SessionPoolName, Lattice as Namespace};
use config;
use chat::Cid;
use super::{ConnectionMessage, PoolMessage};
use super::session::Session;
use super::connection::{NewConnection, Connection};
use super::heap::HeapMap;
use super::lattice::{Lattice, Delta};


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Subscription {
    Pending,
    Session,
}


pub struct Pool {
    name: SessionPoolName,
    channel: Sender<PoolMessage>,
    active_sessions: HeapMap<SessionId, Instant, Session>,
    inactive_sessions: HashMap<SessionId, Session>,

    pending_connections: HashMap<Cid, NewConnection>,
    connections: HashMap<Cid, Connection>,
    topics: HashMap<Topic, HashMap<Cid, Subscription>>,
    lattices: HashMap<Namespace, Lattice>,

    // Setings
    new_connection_timeout: Duration,
}


impl Pool {

    pub fn new(name: SessionPoolName, _cfg: Arc<config::SessionPool>,
        channel: Sender<PoolMessage>)
        -> Pool
    {
        Pool {
            name: name,
            channel: channel,
            active_sessions: HeapMap::new(),
            inactive_sessions: HashMap::new(),
            pending_connections: HashMap::new(),
            connections: HashMap::new(),
            topics: HashMap::new(),
            lattices: HashMap::new(),

            // TODO(tailhook) from config
            new_connection_timeout: Duration::new(60, 0),
        }
    }

    pub fn add_connection(&mut self, conn_id: Cid,
        channel: Sender<ConnectionMessage>)
    {
        let old = self.pending_connections.insert(conn_id,
            NewConnection::new(conn_id, channel));
        debug_assert!(old.is_none());
    }

    pub fn associate(&mut self, conn_id: Cid, session_id: SessionId,
        timestamp: Instant, metadata: Arc<Json>)
    {
        let conn = if let Some(p) = self.pending_connections.remove(&conn_id) {
            p.associate(session_id.clone())
        } else {
            debug!("Connection {:?} does not exist any more", conn_id);
            return;
        };

        for top in &conn.topics {
            let ptr = self.topics
                .get_mut(top)
                .and_then(|m| m.get_mut(&conn_id))
                .expect("topics are consistent");
            *ptr = Subscription::Session;
        }

        let expire = timestamp + self.new_connection_timeout;
        if let Some(mut session) = self.inactive_sessions.remove(&session_id) {
            session.connections.insert(conn_id);
            session.metadata = metadata;
            let val = self.active_sessions.insert(session_id.clone(),
                timestamp, session);
            debug_assert!(val.is_none());
        } else if self.active_sessions.contains_key(&session_id) {
            self.active_sessions.update(&session_id, expire);
            let session = self.active_sessions.get_mut(&session_id).unwrap();
            session.connections.insert(conn_id);
            session.metadata = metadata;
        } else {
            let mut session = Session::new();
            session.connections.insert(conn_id);
            session.metadata = metadata;
            self.active_sessions.insert(session_id.clone(), expire, session);
        }

        let ins = self.connections.insert(conn_id, conn);
        debug_assert!(ins.is_none());
    }

    pub fn del_connection(&mut self, conn_id: Cid) {
        if let Some(conn) = self.pending_connections.remove(&conn_id) {
            for top in &conn.topics {
                unsubscribe(&mut self.topics, top, conn_id);
            }
            return;
        }
        let conn = self.connections.remove(&conn_id)
            .expect("valid connection");
        for topic in &conn.topics {
            unsubscribe(&mut self.topics, &topic, conn_id);
        }
        let session_id = conn.session_id;

        if self.inactive_sessions.contains_key(&session_id) {
            let conns = {
                let mut session = self.inactive_sessions.get_mut(&session_id)
                                  .unwrap();
                session.connections.remove(&conn_id);
                session.connections.len()
            };
            if conns == 0 {
                self.inactive_sessions.remove(&session_id);
            }
        } if let Some(mut session) = self.active_sessions.get_mut(&session_id)
        {
            session.connections.remove(&conn_id);
            // We delete it on inactivation
        }
    }

    pub fn update_activity(&mut self, conn_id: Cid, activity_ts: Instant)
    {
        let sess_id = if let Some(conn) = self.connections.get(&conn_id) {
            &conn.session_id
        } else {
            return;
        };
        if let Some(session) = self.inactive_sessions.remove(sess_id) {
            self.active_sessions.insert(sess_id.clone(), activity_ts, session);
        } else {
            self.active_sessions.update_if_smaller(sess_id, activity_ts)
        }
    }

    pub fn cleanup(&mut self, timestamp: Instant) -> Option<Instant> {
        while self.active_sessions.peek()
            .map(|(_, &x, _)| x < timestamp).unwrap_or(false)
        {
            let (sess_id, _, session) = self.active_sessions.pop().unwrap();
            self.channel.send(PoolMessage::InactiveSession {
                session_id: sess_id.clone(),
                connections_active: session.connections.len(),
                metadata: session.metadata.clone(),
            }).expect("can't send pool message");
            if session.connections.len() == 0 {
                // TODO(tailhook) Maybe do session cleanup ?
            } else {
                let val = self.inactive_sessions.insert(sess_id, session);
                debug_assert!(val.is_none());
            }
            // TODO(tailhook) send inactiity message to IO thread
        }
        self.active_sessions.peek().map(|(_, &x, _)| x)
    }

    pub fn subscribe(&mut self, cid: Cid, topic: Topic) {
        if let Some(conn) = self.connections.get_mut(&cid) {
            conn.topics.insert(topic.clone());
            self.topics.entry(topic).or_insert_with(HashMap::new)
                .insert(cid, Subscription::Session);
        } else if let Some(conn) = self.pending_connections.get_mut(&cid) {
            conn.topics.insert(topic.clone());
            self.topics.entry(topic).or_insert_with(HashMap::new)
                .insert(cid, Subscription::Pending);
        } else {
            debug!("Connection {:?} does not exist any more", cid);
        }
    }

    pub fn unsubscribe(&mut self, cid: Cid, topic: Topic) {
        match unsubscribe(&mut self.topics, &topic, cid) {
            Some(Subscription::Pending) => {
                self.pending_connections.get_mut(&cid)
                    .expect("pending conns and topics are in sync")
                    .topics.remove(&topic);
            }
            Some(Subscription::Session) => {
                self.connections.get_mut(&cid)
                    .expect("pending conns and topics are in sync")
                    .topics.remove(&topic);
            }
            None => {
                debug!("Connection {:?} does not exist any more", cid);
            }
        }
    }

    pub fn publish(&mut self, topic: Topic, data: Arc<Json>) {
        if let Some(cids) = self.topics.get(&topic) {
            for (cid, typ) in cids {
                match *typ {
                    Subscription::Pending => {
                        self.pending_connections.get_mut(cid)
                            .expect("subscriptions out of sync")
                            .message(topic.clone(), data.clone())
                    }
                    Subscription::Session => {
                        self.connections.get_mut(cid)
                            .expect("subscriptions out of sync")
                            .message(topic.clone(), data.clone())
                    }
                }
            }
        }
    }

    pub fn lattice_attach(&mut self, cid: Cid, namespace: Namespace) {
        let conn = if let Some(conn) = self.connections.get_mut(&cid) {
            conn
        } else {
            error!("Attach of {:?} for non-existing connection {:?}",
                   namespace, cid);
            return
        };
        conn.lattices.insert(namespace.clone());
        let lat = if let Some(lat) = self.lattices.get(&namespace) {
            lat
        } else {
            error!("No lattice {:?} at the time of attach (connection {:?})",
                   namespace, cid);
            return
        };
        let mut data = lat.private.get(&conn.session_id)
            .map(|x| x.clone())
            .unwrap_or_else(HashMap::new);
        for (key, values) in &mut data {
            lat.shared.get(key).map(|pubval| {
                values.update(pubval);
            });
        }
        conn.lattice(&namespace, &Arc::new(data));
    }

    pub fn lattice_detach(&mut self, cid: Cid, namespace: Namespace) {
        unimplemented!();
    }

    pub fn lattice_update(&mut self,
        namespace: Namespace, delta: Delta)
    {
        let delta = {
            let lat = self.lattices.entry(namespace.clone())
                .or_insert_with(Lattice::new);

            // Update subscriptions on **original delta**
            for (session_id, rooms) in delta.private.iter() {
                for key in rooms.keys() {
                    lat.subscriptions.entry(key.clone())
                        .or_insert_with(HashSet::new)
                        .insert(session_id.clone());
                }
            }

            // Then make minimal delta
            lat.update(delta)
        };

        // Fighting with borrow checker
        let lat = self.lattices.get(&namespace).unwrap();

        // Send shared-only changes
        let pubdata = Arc::new(delta.shared);
        let mut already_sent = HashSet::new();
        for room in pubdata.keys() {
            if let Some(sessions) = lat.subscriptions.get(room) {
                for sid in sessions {
                    if !already_sent.contains(sid) &&
                       !delta.private.contains_key(sid)
                    {
                        // Can't easily abstract all this away because of borrow checker
                        let sess = if let Some(sess) = self.active_sessions.get(sid) {
                            sess
                        } else if let Some(sess) = self.inactive_sessions.get(sid) {
                            sess
                        } else {
                            continue;
                        };
                        if let Some(connections) = sess.lattices.get(&namespace) {
                            for cid in connections {
                                if let Some(conn) = self.connections.get_mut(cid) {
                                    conn.lattice(&namespace, &pubdata);
                                }
                            }
                        }
                        already_sent.insert(sid.clone());
                    }
                }
            }
        }

        // Send shared *and* private
        for (session_id, mut rooms) in delta.private.into_iter() {
            for (room, values) in rooms.iter_mut() {
                pubdata.get(room).map(|pubval| {
                    values.update(pubval);
                });
            }
            // Can't easily abstract all this away because of borrow checker
            let sess = if let Some(sess) = self.active_sessions.get(&session_id) {
                sess
            } else if let Some(sess) = self.inactive_sessions.get(&session_id) {
                sess
            } else {
                continue;
            };
            if let Some(connections) = sess.lattices.get(&namespace) {
                let update = Arc::new(rooms);
                for cid in connections {
                    if let Some(conn) = self.connections.get_mut(cid) {
                        conn.lattice(&namespace, &update);
                    }
                }
            }
        }
    }
}

fn unsubscribe(topics: &mut HashMap<Topic, HashMap<Cid, Subscription>>,
    topic: &Topic, cid: Cid)
    -> Option<Subscription>
{
    let left = topics.get_mut(topic)
        .map(|set| {
            (set.remove(&cid), set.len())
        });
    left.and_then(|(sub, len)| {
        if len == 0 {
            topics.remove(topic);
        }
        return sub;
    })
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::{Instant, Duration};
    use std::collections::HashMap;
    use rustc_serialize::json::Json;
    use futures::{Async};
    use futures::stream::Stream;
    use futures::sync::mpsc::{unbounded as channel};
    use futures::sync::mpsc::{UnboundedReceiver as Receiver};
    use intern::{SessionId, SessionPoolName, Lattice as Ns};
    use string_intern::{Symbol, Validator};
    use config;
    use chat::Cid;

    use super::Pool;
    use super::super::lattice::{Delta, Values};
    use super::super::{PoolMessage, ConnectionMessage};


    fn pool() -> (Pool, Receiver<PoolMessage>) {
        let (tx, rx) = channel();
        let pool = Pool::new(SessionPoolName::from("test_pool"),
            Arc::new(config::SessionPool {
                listen: config::ListenSocket::Tcp(
                    "127.0.0.1:65535".parse().unwrap()),
                inactivity_handlers: Vec::new(),
            }),
            tx);
        return (pool, rx);
    }

    fn add_u1(pool: &mut Pool) -> (Cid, Receiver<ConnectionMessage>) {
        let cid = Cid::new();
        let (tx, rx) = channel();
        pool.add_connection(cid, tx);
        pool.associate(cid, SessionId::from("user1"), Instant::now(),
            Arc::new(Json::Object(vec![
                ("user_id", "user1"),
            ].into_iter().map(|(x, y)| {
                (x.into(), Json::String(y.into()))
            }).collect())));
        return (cid, rx);
    }

    fn add_u2(pool: &mut Pool) -> (Cid, Receiver<ConnectionMessage>) {
        let cid = Cid::new();
        let (tx, rx) = channel();
        pool.add_connection(cid, tx);
        pool.associate(cid, SessionId::from("user2"), Instant::now(),
            Arc::new(Json::Object(vec![
                ("user_id", "user2"),
            ].into_iter().map(|(x, y)| {
                (x.into(), Json::String(y.into()))
            }).collect())));
        return (cid, rx);
    }

    #[test]
    fn add_conn() {
        let (mut pool, _rx) = pool();
        add_u1(&mut pool);
    }

    fn get_item<S: Stream>(s: &mut S) -> S::Item {
        match s.poll().map_err(|_|{}).expect("stream error") {
            Async::Ready(v) => v.expect("stream eof"),
            Async::NotReady => panic!("stream not ready"),
        }
    }

    #[test]
    fn disconnect_after_inactive() {
        let (mut pool, mut rx) = pool();
        let (cid, _) = add_u1(&mut pool);
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(10, 0));
        // New connection timeout is expected to be ~ 60 seconds
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(120, 0));
        assert!(matches!(get_item(&mut rx),
            PoolMessage::InactiveSession { ref session_id, ..}
            if *session_id == SessionId::from("user1")));
        assert_eq!(pool.active_sessions.len(), 0);
        assert_eq!(pool.inactive_sessions.len(), 1);
        pool.del_connection(cid);
        assert_eq!(pool.active_sessions.len(), 0);
        assert_eq!(pool.inactive_sessions.len(), 0);
    }

    #[test]
    fn disconnect_before_inactive() {
        let (mut pool, mut rx) = pool();
        let (cid, _) = add_u1(&mut pool);
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(10, 0));
        // New connection timeout is expected to be ~ 60 seconds
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.del_connection(cid);
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(120, 0));
        assert!(matches!(get_item(&mut rx),
            PoolMessage::InactiveSession { ref session_id, ..}
            if *session_id == SessionId::from("user1")));
        assert_eq!(pool.active_sessions.len(), 0);
        assert_eq!(pool.inactive_sessions.len(), 0);
    }

    trait Builder {
        type Key;
        type Value;
        fn new() -> Self;
        fn add(self, key: &'static str, val: Self::Value) -> Self;
    }

    impl<S: Validator, V> Builder for HashMap<Symbol<S>, V> {
        type Key = Symbol<S>;
        type Value = V;
        fn new() -> HashMap<Symbol<S>, V> {
            HashMap::new()
        }
        fn add(mut self, key: &'static str, val: V) -> Self {
            self.insert(Symbol::from(key), val);
            self
        }
    }

    fn builder<S: Validator, V>() -> HashMap<Symbol<S>, V> {
        HashMap::new()
    }

    #[test]
    fn empty_lattices() {
        let (mut pool, _rx) = pool();
        let lat = Ns::from("rooms");
        let (c1, mut rx1) = add_u1(&mut pool);
        pool.lattice_update(lat.clone(), Delta {
            shared: builder()
                    .add("room1", Values::new()),
            private: builder()
                .add("user1", builder()
                    .add("room1", Values::new())),
        });
        pool.lattice_attach(c1, lat.clone());
        assert_matches!(get_item(&mut rx1),
            ConnectionMessage::Lattice(..)); // TODO(tailhook)
        let (c2, mut rx2) = add_u2(&mut pool);
        pool.lattice_update(lat.clone(), Delta {
            shared: builder()
                    .add("room2", Values::new()),
            private: builder()
                    .add("user2", builder()
                        .add("room2", Values::new())),
        });
        pool.lattice_attach(c2, lat.clone());
        assert_matches!(get_item(&mut rx2),
            ConnectionMessage::Lattice(..)); // TODO(tailhook)
        pool.lattice_update(lat.clone(), Delta {
            shared: builder()
                    // TODO(tailhook)
                    .add("room1", Values::new())
                    .add("room2", Values::new()),
            private: builder(),
        });
        // TODO(tailhook)
    }
}
