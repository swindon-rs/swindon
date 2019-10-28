use tokio_core::reactor::Handle;

use crate::chat;
use crate::config::ConfigCell;
use crate::handlers::files;
use crate::http_pools::HttpPools;
use self_meter_http::Meter;
use crate::request_id::RequestId;
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
