use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::HashMap;

use rustc_serialize::json::Json;

use intern::Atom;
use config;
use chat::Cid;
use super::session::Session;
use super::connection::{NewConnection, Connection};
use super::heap::HeapMap;


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Subscription {
    Pending,
    Session,
}


pub struct Pool {
    name: Atom,
    active_sessions: HeapMap<Atom, Instant, Session>,
    inactive_sessions: HashMap<Atom, Session>,

    pending_connections: HashMap<Cid, NewConnection>,
    connections: HashMap<Cid, Connection>,
    topics: HashMap<Atom, HashMap<Cid, Subscription>>,

    // Setings
    new_connection_timeout: Duration,
}


impl Pool {

    pub fn new(name: Atom, _cfg: Arc<config::SessionPool>) -> Pool {
        Pool {
            name: name,
            active_sessions: HeapMap::new(),
            inactive_sessions: HashMap::new(),
            pending_connections: HashMap::new(),
            connections: HashMap::new(),
            topics: HashMap::new(),

            // TODO(tailhook) from config
            new_connection_timeout: Duration::new(60, 0),
        }
    }

    pub fn add_connection(&mut self, conn_id: Cid) {
        let old = self.pending_connections.insert(conn_id,
            NewConnection::new(conn_id));
        debug_assert!(old.is_none());
    }

    pub fn associate(&mut self, conn_id: Cid, session_id: Atom,
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

    pub fn unsubscribe(&mut self, cid: Cid, topic: Atom) {
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

    pub fn publish(&mut self, topic: Atom, data: Arc<Json>) {
        if let Some(cids) = self.topics.get(&topic) {
            for (cid, typ) in cids {
                match *typ {
                    Subscription::Pending => {
                        self.pending_connections.get_mut(cid)
                            .expect("subscriptions out of sync")
                            .message(data.clone())
                    }
                    Subscription::Session => {
                        self.connections.get(cid)
                            .expect("subscriptions out of sync")
                            .message(data.clone())
                    }
                }
            }
        }
    }

}

fn unsubscribe(topics: &mut HashMap<Atom, HashMap<Cid, Subscription>>,
    topic: &Atom, cid: Cid)
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
        let cid = Cid::new();
        pool.add_connection(cid);
        pool.associate(cid, Atom::from("user1"), Instant::now(),
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
        pool.add_connection(cid);
        pool.associate(cid, Atom::from("user1"), Instant::now(),
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
        pool.add_connection(cid);
        pool.associate(cid, Atom::from("user1"), Instant::now(),
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
