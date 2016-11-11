use std::sync::Arc;
use std::time::{Instant, Duration};

use rustc_serialize::json::Json;

use intern::Atom;
use config;
use chat::Cid;
use super::session::Session;
use super::heap::HeapMap;


pub struct Pool {
    name: Atom,
    sessions: HeapMap<Atom, Instant, Session>,
    new_connection_timeout: Duration,
}


impl Pool {

    pub fn new(name: Atom, _cfg: Arc<config::SessionPool>) -> Pool {
        Pool {
            name: name,
            // TODO(tailhook) from config
            new_connection_timeout: Duration::new(0, 60),
            sessions: HeapMap::new(),
        }
    }

    pub fn add_connection(&mut self, timestamp: Instant,
        user_id: Atom, conn_id: Cid, metadata: Arc<Json>)
    {
        let expire = timestamp + self.new_connection_timeout;
        if self.sessions.contains_key(&user_id) {
            self.sessions.update(&user_id, expire);
            let session = self.sessions.get_mut(&user_id).unwrap();
            session.connections.insert(conn_id);
            session.metadata = metadata;
        } else {
            let mut session = Session::new();
            session.connections.insert(conn_id);
            session.metadata = metadata;
            self.sessions.insert(user_id, expire, session);
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::Instant;
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
}
