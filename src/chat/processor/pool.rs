use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry::Occupied;

use serde_json::Value as Json;
use futures::sync::mpsc::{UnboundedSender as Sender};

use intern::{Topic, SessionId, SessionPoolName, Lattice as Namespace};
use config;
use chat::{Cid, CloseReason, ConnectionSender};
use super::{ConnectionMessage, PoolMessage};
use super::session::Session;
use super::connection::{NewConnection, Connection};
use super::heap::HeapMap;
use super::lattice::{Lattice, Delta};
use metrics::{Integer, Counter};

lazy_static! {
    pub static ref ACTIVE_SESSIONS: Integer = Integer::new();
    pub static ref INACTIVE_SESSIONS: Integer = Integer::new();

    pub static ref PUBSUB_INPUT: Counter = Counter::new();
    pub static ref PUBSUB_OUTPUT: Counter = Counter::new();
    pub static ref TOPICS: Integer = Integer::new();

    pub static ref LATTICES: Integer = Integer::new();
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Subscription {
    Pending,
    Session,
}

pub struct Sessions {
    active: HeapMap<SessionId, Instant, Session>,
    inactive: HashMap<SessionId, Session>,
}

pub struct Pool {
    name: SessionPoolName,
    channel: Sender<PoolMessage>,
    sessions: Sessions,

    pending_connections: HashMap<Cid, NewConnection>,
    connections: HashMap<Cid, Connection>,
    topics: HashMap<Topic, HashMap<Cid, Subscription>>,
    lattices: HashMap<Namespace, Lattice>,

    // Setings
    new_connection_timeout: Duration,
}

impl Sessions {
    fn new() -> Sessions {
        Sessions {
            active: HeapMap::new(),
            inactive: HashMap::new(),
        }
    }

    fn get_mut(&mut self, id: &SessionId) -> Option<&mut Session> {
        if let Some(sess) = self.active.get_mut(id) {
            Some(sess)
        } else if let Some(sess) = self.inactive.get_mut(id) {
            Some(sess)
        } else {
            None
        }
    }

    fn get(&self, id: &SessionId) -> Option<&Session> {
        if let Some(sess) = self.active.get(id) {
            Some(sess)
        } else if let Some(sess) = self.inactive.get(id) {
            Some(sess)
        } else {
            None
        }
    }
}

impl Pool {

    pub fn new(name: SessionPoolName, cfg: Arc<config::SessionPool>,
        channel: Sender<PoolMessage>)
        -> Pool
    {
        Pool {
            name: name,
            channel: channel,
            sessions: Sessions::new(),
            pending_connections: HashMap::new(),
            connections: HashMap::new(),
            topics: HashMap::new(),
            lattices: HashMap::new(),
            new_connection_timeout: (*cfg.new_connection_idle_timeout).clone(),
        }
    }

    pub fn add_connection(&mut self, conn_id: Cid,
        channel: ConnectionSender)
    {
        let old = self.pending_connections.insert(conn_id,
            NewConnection::new(conn_id, channel));
        debug!("Add new connection {:?}", conn_id);
        debug_assert!(old.is_none());
    }

    pub fn associate(&mut self, conn_id: Cid, session_id: SessionId,
        timestamp: Instant, metadata: Arc<Json>)
    {
        let mut conn =
            if let Some(p) = self.pending_connections.remove(&conn_id) {
                p.associate(session_id.clone())
            } else {
                debug!("Connection {:?} does not exist any more", conn_id);
                return;
            };

        for namespace in &conn.lattices {
            if let Some(lat) = self.lattices.get(namespace) {
                lattice_from(&mut conn.channel, namespace, &session_id, lat);
            } else {
                error!("No lattice {:?} at the time \
                    of connection association (connection {:?})",
                    namespace, conn_id);
                return
            };
        }

        for top in &conn.topics {
            let ptr = self.topics
                .get_mut(top)
                .and_then(|m| m.get_mut(&conn_id))
                .expect("topics are consistent");
            *ptr = Subscription::Session;
        }

        let expire = timestamp + self.new_connection_timeout;
        if let Some(mut session) = self.sessions.inactive.remove(&session_id) {
            session.connections.insert(conn_id);
            session.metadata = metadata;
            copy_attachments(&mut session, &conn.lattices, conn_id);
            let val = self.sessions.active.insert(session_id.clone(),
                timestamp, session);
            debug_assert!(val.is_none());
            INACTIVE_SESSIONS.decr(1);
            ACTIVE_SESSIONS.incr(1);
        } else if self.sessions.active.contains_key(&session_id) {
            self.sessions.active.update(&session_id, expire);
            let mut session = self.sessions.active.get_mut(&session_id).unwrap();
            session.connections.insert(conn_id);
            session.metadata = metadata;
            copy_attachments(&mut session, &conn.lattices, conn_id);
        } else {
            let mut session = Session::new();
            session.connections.insert(conn_id);
            session.metadata = metadata;
            copy_attachments(&mut session, &conn.lattices, conn_id);
            self.sessions.active.insert(session_id.clone(), expire, session);
            ACTIVE_SESSIONS.incr(1);
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

        if self.sessions.inactive.contains_key(&session_id) {
            let conns = {
                let mut session = self.sessions.inactive.get_mut(&session_id)
                                  .unwrap();
                session.connections.remove(&conn_id);
                for lat in &conn.lattices {
                    remove_lattice(session, &session_id,
                                   conn_id, &mut self.lattices, lat, false);
                }
                session.connections.len()
            };
            if conns == 0 {
                if self.sessions.inactive.remove(&session_id).is_some() {
                    INACTIVE_SESSIONS.decr(1);
                }
            }
        } if let Some(mut session) = self.sessions.active.get_mut(&session_id)
        {
            session.connections.remove(&conn_id);
            // We delete it on inactivation
        }
    }

    pub fn update_activity(&mut self, sess_id: SessionId, activity_ts: Instant)
    {
        if let Some(session) = self.sessions.inactive.remove(&sess_id) {
            INACTIVE_SESSIONS.decr(1);
            ACTIVE_SESSIONS.incr(1);
            self.sessions.active.insert(sess_id.clone(), activity_ts, session);
        } else {
            self.sessions.active.update_if_smaller(&sess_id, activity_ts)
        }
    }

    pub fn cleanup(&mut self, timestamp: Instant) -> Option<Instant> {
        while self.sessions.active.peek()
            .map(|(_, &x, _)| x < timestamp).unwrap_or(false)
        {
            let (sess_id, _, session) = self.sessions.active.pop().unwrap();
            ACTIVE_SESSIONS.decr(1);
            self.channel.send(PoolMessage::InactiveSession {
                session_id: sess_id.clone(),
                connections_active: session.connections.len(),
                metadata: session.metadata.clone(),
            }).expect("can't send pool message");
            if session.connections.len() == 0 {
                // TODO(tailhook) Maybe do session cleanup ?
            } else {
                let val = self.sessions.inactive.insert(sess_id, session);
                INACTIVE_SESSIONS.incr(1);
                debug_assert!(val.is_none());
            }
        }
        self.sessions.active.peek().map(|(_, &x, _)| x)
    }

    pub fn subscribe(&mut self, cid: Cid, topic: Topic) {
        if let Some(conn) = self.connections.get_mut(&cid) {
            conn.topics.insert(topic.clone());
            self.topics.entry(topic)
                .or_insert_with(|| {
                    TOPICS.incr(1);
                    HashMap::new()
                })
                .insert(cid, Subscription::Session);
        } else if let Some(conn) = self.pending_connections.get_mut(&cid) {
            conn.topics.insert(topic.clone());
            self.topics.entry(topic)
                .or_insert_with(|| {
                    TOPICS.incr(1);
                    HashMap::new()
                })
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
        PUBSUB_INPUT.incr(1);
        if let Some(cids) = self.topics.get(&topic) {
            for (cid, typ) in cids {
                PUBSUB_OUTPUT.incr(1);
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
        } else if let Some(conn) = self.pending_connections.get_mut(&cid) {
            conn.lattices.insert(namespace);
            return
        } else {
            info!("Attach of {:?} for non-existing connection {:?}",
                   namespace, cid);
            return
        };

        let sess = if let Some(sess) = self.sessions.get_mut(&conn.session_id)
            {
                sess
            } else {
                error!("Connection {:?} doesn't have corresponding \
                    session {:?}", cid, conn.session_id);
                return
            };
        sess.lattices.entry(namespace.clone())
            .or_insert_with(HashSet::new)
            .insert(cid);

        conn.lattices.insert(namespace.clone());

        if let Some(lat) = self.lattices.get(&namespace) {
            lattice_from(&mut conn.channel, &namespace, &conn.session_id, lat);
        } else {
            error!("No lattice {:?} at the time of attach (connection {:?})",
                   namespace, cid);
            return
        };
    }

    pub fn lattice_detach(&mut self, cid: Cid, namespace: Namespace) {
        let conn = if let Some(conn) = self.connections.get_mut(&cid) {
            conn
        } else if let Some(conn) = self.pending_connections.get_mut(&cid) {
            conn.lattices.remove(&namespace);
            return
        } else {
            info!("Detach of {:?} for non-existing connection {:?}",
                   namespace, cid);
            return
        };

        let sess = if let Some(sess) = self.sessions.get_mut(&conn.session_id)
            {
                sess
            } else {
                error!("Connection {:?} doesn't have corresponding \
                    session {:?}", cid, conn.session_id);
                return
            };

        remove_lattice(sess, &conn.session_id,
                       cid, &mut self.lattices, &namespace, true);

        conn.lattices.remove(&namespace);

        // TODO(tailhook) maybe cleanup lattices now
    }

    pub fn lattice_update(&mut self,
        namespace: Namespace, delta: Delta)
    {
        let delta = {
            let lat = self.lattices.entry(namespace.clone())
                .or_insert_with(|| {
                    LATTICES.incr(1);
                    Lattice::new()
                });

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
                    if already_sent.contains(sid) ||
                       delta.private.contains_key(sid)
                    {
                        continue;
                    }
                    // Can't easily abstract all this away because of
                    // the borrow checker
                    let sess = if let Some(sess) = self.sessions.get(sid) {
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

        // Send shared *and* private
        for (session_id, mut rooms) in delta.private.into_iter() {
            for (room, values) in rooms.iter_mut() {
                pubdata.get(room).map(|pubval| {
                    values.update(pubval);
                });
            }
            // Can't easily abstract all this away because of borrow checker
            let sess = if let Some(sess) = self.sessions.get(&session_id) {
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
    pub fn stop(self) {
        for (_, mut conn) in self.pending_connections {
            conn.stop(CloseReason::PoolStopped);
        }
        for (_, mut conn) in self.connections {
            conn.stop(CloseReason::PoolStopped);
        }
    }

}

fn remove_lattice(session: &mut Session, session_id: &SessionId,
                  cid: Cid, lattices: &mut HashMap<Namespace, Lattice>,
                  namespace: &Namespace, absent_is_ok: bool)
{
    if let Occupied(mut e) = session.lattices.entry(namespace.clone()) {
        e.get_mut().remove(&cid);
        if e.get().len() == 0 {
            e.remove_entry();
            // TODO(tailhook) cleanup keys from lattice
            if let Occupied(mut lat) = lattices.entry(namespace.clone()) {
                lat.get_mut().remove_session(session_id);
                if lat.get().is_empty() {
                    lat.remove_entry();
                    LATTICES.decr(1);
                }
            }
        }
    } else {
        if absent_is_ok {
            info!("Never subscribed {:?} in {:?} to {:?}",
                  cid, session_id, namespace);
        } else {
            // should we crash here?
            error!("Never subscribed {:?} in {:?} to {:?}",
                  cid, session_id, namespace);
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
            if topics.remove(topic).is_some() {
                TOPICS.decr(1);
            }
        }
        return sub;
    })
}

fn lattice_from(channel: &mut ConnectionSender,
    namespace: &Namespace, session: &SessionId, lattice: &Lattice)
{
    let mut data = lattice.private.get(session)
        .map(|x| x.clone())
        .unwrap_or_else(HashMap::new);
    for (key, values) in &mut data {
        lattice.shared.get(key).map(|pubval| {
            values.update(pubval);
        });
    }
    let msg = ConnectionMessage::Lattice(namespace.clone(), Arc::new(data));
    channel.send(msg);
}

fn copy_attachments(sess: &mut Session, list: &HashSet<Namespace>, cid: Cid) {
    for namespace in list {
        sess.lattices.entry(namespace.clone())
            .or_insert_with(HashSet::new)
            .insert(cid);
    }
}


#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::{Instant, Duration};
    use std::collections::HashMap;
    use futures::{Async};
    use futures::stream::Stream;
    use futures::sync::mpsc::{unbounded as channel};
    use futures::sync::mpsc::{UnboundedReceiver as Receiver};
    use intern::{SessionId, SessionPoolName, Lattice as Ns};
    use quire::De;

    use string_intern::{Symbol, Validator};
    use config;
    use chat::{Cid, ConnectionSender};

    use super::Pool;
    use super::super::lattice::{Delta, Values};
    use super::super::{PoolMessage, ConnectionMessage};


    fn pool() -> (Pool, Receiver<PoolMessage>) {
        let (tx, rx) = channel();
        let pool = Pool::new(SessionPoolName::from("test_pool"),
            Arc::new(config::SessionPool {
                listen: vec![
                    config::ListenSocket::Tcp(
                    "127.0.0.1:65535".parse().unwrap())],
                inactivity_handlers: Vec::new(),
                new_connection_idle_timeout: De::new(Duration::from_secs(60)),
                client_min_idle_timeout: De::new(Duration::from_secs(60)),
                client_max_idle_timeout: De::new(Duration::from_secs(60)),
                client_default_idle_timeout: De::new(Duration::from_secs(60)),
                max_connections: 100,
                listen_error_timeout: Duration::from_secs(1).into(),
                pipeline_depth: 2,
                max_payload_size: 10_000_000,
            }),
            tx);
        return (pool, rx);
    }

    fn add_u1(pool: &mut Pool) -> (Cid, Receiver<ConnectionMessage>) {
        let cid = Cid::new();
        let (tx, rx) = ConnectionSender::new();
        pool.add_connection(cid, tx);
        pool.associate(cid, SessionId::from("user1"), Instant::now(),
            Arc::new(json!({"user_id": "user1"})));
        return (cid, rx);
    }

    fn add_u2(pool: &mut Pool) -> (Cid, Receiver<ConnectionMessage>) {
        let cid = Cid::new();
        let (tx, rx) = ConnectionSender::new();
        pool.add_connection(cid, tx);
        pool.associate(cid, SessionId::from("user2"), Instant::now(),
            Arc::new(json!({"user_id": "user2"})));
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
        assert_eq!(pool.sessions.active.len(), 1);
        assert_eq!(pool.sessions.inactive.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(10, 0));
        // New connection timeout is expected to be ~ 60 seconds
        assert_eq!(pool.sessions.active.len(), 1);
        assert_eq!(pool.sessions.inactive.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(120, 0));
        assert!(matches!(get_item(&mut rx),
            PoolMessage::InactiveSession { ref session_id, ..}
            if *session_id == SessionId::from("user1")));
        assert_eq!(pool.sessions.active.len(), 0);
        assert_eq!(pool.sessions.inactive.len(), 1);
        pool.del_connection(cid);
        assert_eq!(pool.sessions.active.len(), 0);
        assert_eq!(pool.sessions.inactive.len(), 0);
    }

    #[test]
    fn disconnect_before_inactive() {
        let (mut pool, mut rx) = pool();
        let (cid, _) = add_u1(&mut pool);
        assert_eq!(pool.sessions.active.len(), 1);
        assert_eq!(pool.sessions.inactive.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(10, 0));
        // New connection timeout is expected to be ~ 60 seconds
        assert_eq!(pool.sessions.active.len(), 1);
        assert_eq!(pool.sessions.inactive.len(), 0);
        pool.del_connection(cid);
        assert_eq!(pool.sessions.active.len(), 1);
        assert_eq!(pool.sessions.inactive.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(120, 0));
        assert!(matches!(get_item(&mut rx),
            PoolMessage::InactiveSession { ref session_id, ..}
            if *session_id == SessionId::from("user1")));
        assert_eq!(pool.sessions.active.len(), 0);
        assert_eq!(pool.sessions.inactive.len(), 0);
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
    fn attach_before_associate() {
        let (mut pool, _rx) = pool();
        let cid = Cid::new();
        let (tx, mut rx) = ConnectionSender::new();
        pool.add_connection(cid, tx);
        pool.lattice_update(Ns::from("rooms"), Delta {
            shared: builder(),
            private: builder().add("user1", builder()),
        });
        pool.lattice_attach(cid, Ns::from("rooms"));
        pool.associate(cid, SessionId::from("user1"), Instant::now(),
            Arc::new(json!({"user_id": "user1"})));
        assert_matches!(get_item(&mut rx),
            ConnectionMessage::Lattice(..)); // TODO(tailhook)
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
