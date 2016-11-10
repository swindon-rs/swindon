use std::sync::Arc;

use config;


pub struct Pool {

}


impl Pool {
    pub fn new(_cfg: Arc<config::SessionPool>) -> Pool {
        Pool {
        }
    }
}
