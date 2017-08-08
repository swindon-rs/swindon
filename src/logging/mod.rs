
mod context;
pub mod http;

pub use self::context::AsContext;


use std::io::{stdout, Write};
use std::sync::Arc;

use runtime::Runtime;


pub fn log<C: AsContext>(runtime: &Arc<Runtime>, ctx: C) {
    let cfg = runtime.config.get();
    if cfg.debug_logging {
        if let Some(ref fmt) = cfg.log_formats.get("debug-log") {
            let ctx = ctx.as_context();
            match fmt.template.render(&ctx) {
                Ok(mut line) => {
                    line.push('\n');
                    stdout().write_all(line.as_bytes())
                        .map_err(|e| {
                            warn!("Can't write debug log: {}", e)
                        }).ok();
                }
                Err(e) => {
                    warn!("Can't log request: {:?}", e);
                }
            };
        }
    }
}
