use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::HashMap;

use rustc_serialize::json::Json;

use intern::Atom;
use config;
use chat::Cid;
use super::session::Session;
use super::heap::HeapMap;


pub struct Pool {
    name: Atom,
    active_sessions: HeapMap<Atom, Instant, Session>,
    inactive_sessions: HashMap<Atom, Session>,
    new_connection_timeout: Duration,
}


impl Pool {

    pub fn new(name: Atom, _cfg: Arc<config::SessionPool>) -> Pool {
        Pool {
            name: name,
            // TODO(tailhook) from config
            new_connection_timeout: Duration::new(60, 0),
            active_sessions: HeapMap::new(),
            inactive_sessions: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, timestamp: Instant,
        user_id: Atom, conn_id: Cid, metadata: Arc<Json>)
    {
        let expire = timestamp + self.new_connection_timeout;
        if let Some(mut session) = self.inactive_sessions.remove(&user_id) {
            session.connections.insert(conn_id);
            session.metadata = metadata;
            let val = self.active_sessions.insert(user_id, timestamp, session);
            debug_assert!(val.is_none());
        } else if self.active_sessions.contains_key(&user_id) {
            self.active_sessions.update(&user_id, expire);
            let session = self.active_sessions.get_mut(&user_id).unwrap();
            session.connections.insert(conn_id);
            session.metadata = metadata;
        } else {
            let mut session = Session::new();
            session.connections.insert(conn_id);
            session.metadata = metadata;
            self.active_sessions.insert(user_id, expire, session);
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
            let val = self.inactive_sessions.insert(user_id, session);
            debug_assert!(val.is_none());
        }
        self.active_sessions.peek().map(|(_, &x, _)| x)
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
    fn cleanup() {
        let mut pool = pool();
        pool.add_connection(Instant::now(), Atom::from("user1"), Cid::new(),
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
    }
}
