


pub fn early_error(rt: &Arc<Runtime>, status: Status, debug: &Debug) {
    let cfg = rt.config.get();
    if cfg.debug_logging {
        if let Some(ref fmt) = cfg.log_formats.get("debug-log") {
            EarlyContext {
                request: EarlyRequest {
                },
                request: EarlyResponse {
                    status: status,
                },
            }.log(fmt, BufWriter::new(stdout()));
        }
    }
}
