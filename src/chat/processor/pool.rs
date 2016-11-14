use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::{HashMap, HashSet};

use rustc_serialize::json::Json;

use intern::Atom;
use config;
use chat::Cid;
use super::session::Session;
use super::connection::Connection;
use super::heap::HeapMap;


pub struct Pool {
    name: Atom,
    active_sessions: HeapMap<Atom, Instant, Session>,
    inactive_sessions: HashMap<Atom, Session>,
    connections: HashMap<Cid, Connection>,
    topics: HashMap<Atom, HashSet<Cid>>,

    // Setings
    new_connection_timeout: Duration,
}


impl Pool {

    pub fn new(name: Atom, _cfg: Arc<config::SessionPool>) -> Pool {
        Pool {
            name: name,
            active_sessions: HeapMap::new(),
            inactive_sessions: HashMap::new(),
            connections: HashMap::new(),
            topics: HashMap::new(),

            // TODO(tailhook) from config
            new_connection_timeout: Duration::new(60, 0),
        }
    }

    pub fn add_connection(&mut self, timestamp: Instant,
        session_id: Atom, conn_id: Cid, metadata: Arc<Json>)
    {
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

        let ins = self.connections.insert(conn_id,
            Connection::new(conn_id, session_id));
        debug_assert!(ins.is_none());
    }

    pub fn del_connection(&mut self, conn_id: Cid) {
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

    pub fn update_activity(&mut self, user_id: Atom, activity_ts: Instant)
    {
        if let Some(session) = self.inactive_sessions.remove(&user_id) {
            self.active_sessions.insert(user_id, activity_ts, session);
        } else {
            self.active_sessions.update_if_smaller(&user_id, activity_ts)
        }
    }

    pub fn cleanup(&mut self, timestamp: Instant) -> Option<Instant> {
        while self.active_sessions.peek()
            .map(|(_, &x, _)| x < timestamp).unwrap_or(false)
        {
            let (user_id, _, session) = self.active_sessions.pop().unwrap();
            if session.connections.len() == 0 {
                // TODO(tailhook) Maybe do session cleanup ?
            } else {
                let val = self.inactive_sessions.insert(user_id, session);
                debug_assert!(val.is_none());
            }
            // TODO(tailhook) send inactiity message to IO thread
        }
        self.active_sessions.peek().map(|(_, &x, _)| x)
    }

    pub fn subscribe(&mut self, cid: Cid, topic: Atom) {
        self.connections.get_mut(&cid)
            // TODO(tailhook) Should we allow invalid connections?
            .expect("valid cid")
            .topics.insert(topic.clone());
        self.topics.entry(topic).or_insert_with(HashSet::new)
            .insert(cid);
    }

    pub fn unsubscribe(&mut self, cid: Cid, topic: Atom) {
        self.connections.get_mut(&cid)
            // TODO(tailhook) Should we allow invalid connections?
            .expect("valid cid")
            .topics.remove(&topic);
        unsubscribe(&mut self.topics, &topic, cid);
    }

    pub fn publish(&mut self, topic: Atom, data: Arc<Json>) {
        if let Some(cids) = self.topics.get(&topic) {
            for cid in cids {
                self.connections.get(cid)
                    .expect("subscriptions out of sync")
                    .message(data.clone())
            }
        }
    }

}

fn unsubscribe(topics: &mut HashMap<Atom, HashSet<Cid>>,
    topic: &Atom, cid: Cid)
{
    let left = topics.get_mut(topic)
        .map(|set| {
            set.remove(&cid);
            set.len()
        });
    if left == Some(0) {
        topics.remove(topic);
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::{Instant, Duration};
    use rustc_serialize::json::Json;
    use intern::Atom;
    use config;
    use chat::Cid;

    use super::Pool;

    fn pool() -> Pool {
        Pool::new(Atom::from("test_pool"), Arc::new(config::SessionPool {
            listen: config::ListenSocket::Tcp(
                "127.0.0.1:65535".parse().unwrap())
        }))
    }

    #[test]
    fn add_conn() {
        let mut pool = pool();
        pool.add_connection(Instant::now(), Atom::from("user1"), Cid::new(),
            Arc::new(Json::Object(vec![
                ("user_id", "user1"),
            ].into_iter().map(|(x, y)| {
                (x.into(), Json::String(y.into()))
            }).collect())));
    }

    #[test]
    fn disconnect_after_inactive() {
        let mut pool = pool();
        let cid = Cid::new();
        pool.add_connection(Instant::now(), Atom::from("user1"), cid,
            Arc::new(Json::Object(vec![
                ("user_id", "user1"),
            ].into_iter().map(|(x, y)| {
                (x.into(), Json::String(y.into()))
            }).collect())));
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(10, 0));
        // New connection timeout is expected to be ~ 60 seconds
        assert_eq!(pool.active_sessions.len(), 1);
        assert_eq!(pool.inactive_sessions.len(), 0);
        pool.cleanup(Instant::now() + Duration::new(120, 0));
        assert_eq!(pool.active_sessions.len(), 0);
        assert_eq!(pool.inactive_sessions.len(), 1);
        pool.del_connection(cid);
        assert_eq!(pool.active_sessions.len(), 0);
        assert_eq!(pool.inactive_sessions.len(), 0);
    }

    #[test]
    fn disconnect_before_inactive() {
        let mut pool = pool();
        let cid = Cid::new();
        pool.add_connection(Instant::now(), Atom::from("user1"), cid,
            Arc::new(Json::Object(vec![
                ("user_id", "user1"),
            ].into_iter().map(|(x, y)| {
                (x.into(), Json::String(y.into()))
            }).collect())));
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
        assert_eq!(pool.active_sessions.len(), 0);
        assert_eq!(pool.inactive_sessions.len(), 0);
    }
}
