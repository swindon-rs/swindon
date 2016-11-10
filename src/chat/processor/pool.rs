use std::sync::Arc;
use std::collections::{HashMap, HashSet};

use rustc_serialize::json::Json;

use intern::Atom;
use config;
use chat::Cid;

pub struct Session {
    connections: HashSet<Cid>,
    metadata: Arc<Json>,
}

pub struct Pool {
    name: Atom,
    sessions: HashMap<Atom, Session>,
}


impl Pool {

    pub fn new(name: Atom, _cfg: Arc<config::SessionPool>) -> Pool {
        Pool {
            name: name,
            sessions: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self,
        user_id: Atom, conn_id: Cid, metadata: Arc<Json>)
    {
        let entry = self.sessions.entry(user_id).or_insert_with(|| Session {
                connections: HashSet::new(),
                metadata: metadata.clone(),
        });
        entry.connections.insert(conn_id);
        // TODO(tailhook) should we merge metadata
        entry.metadata = metadata;
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
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
        pool.add_connection(Atom::from("user1"), Cid::new(),
            Arc::new(Json::Object(vec![
                ("user_id", "user1"),
            ].into_iter().map(|(x, y)| {
                (x.into(), Json::String(y.into()))
            }).collect())));
    }
}
