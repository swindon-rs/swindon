use std::sync::{Arc, RwLock};

use tokio_core::reactor::Handle;

use chat;
use config::ConfigCell;
use http_pools::HttpPools;


pub struct Runtime {
    pub config: ConfigCell,
    pub handle: Handle,
    pub http_pools: HttpPools,
    pub chat_processor: Arc<RwLock<chat::Processor>>,
}
