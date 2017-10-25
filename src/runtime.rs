use tokio_core::reactor::Handle;

use chat;
use config::ConfigCell;
use handlers::files;
use http_pools::HttpPools;
use self_meter_http::Meter;
use request_id::RequestId;
use ns_router::Router;


pub struct Runtime {
    pub config: ConfigCell,
    pub handle: Handle,
    pub http_pools: HttpPools,
    pub session_pools: chat::SessionPools,
    pub disk_pools: files::DiskPools,
    pub meter: Meter,
    pub server_id: ServerId,
    pub resolver: Router,
}

/// Runtime server identifier.
/// Used mainly in chat.
pub type ServerId = RequestId;
