use tokio_core::reactor::Handle;

use chat;
use config::ConfigCell;
use http_pools::HttpPools;
use self_meter_http::Meter;


pub struct Runtime {
    pub config: ConfigCell,
    pub handle: Handle,
    pub http_pools: HttpPools,
    pub session_pools: chat::SessionPools,
    pub meter: Meter,
}
