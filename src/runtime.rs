use tokio_core::reactor::Handle;

use config::ConfigCell;
use http_pools::HttpPools;


pub struct Runtime {
    pub config: ConfigCell,
    pub handle: Handle,
    pub http_pools: HttpPools,
    //pub chat_processor:
}
