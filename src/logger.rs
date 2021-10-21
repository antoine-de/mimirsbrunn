use slog::{self, o, slog_o, Drain, Never};
use std::env;

pub fn logger_init() -> (slog_scope::GlobalLoggerGuard, ()) {
    if let Ok(s) = env::var("RUST_LOG_JSON") {
        let mut drain = slog_json::Json::new(std::io::stderr())
            .add_default_keys()
            .add_key_value(o!(
                        "module" => slog::FnValue(|rinfo : &slog::Record<'_>| {
                            rinfo.module()
                        })
            ));
        if s == "pretty" {
            drain = drain.set_pretty(true);
        }
        configure_logger(drain.build().fuse())
    } else {
        configure_logger(
            slog_term::CompactFormat::new(slog_term::PlainDecorator::new(std::io::stderr()))
                .build()
                .fuse(),
        )
    }
}

fn configure_logger<T>(drain: T) -> (slog_scope::GlobalLoggerGuard, ())
where
    T: Drain<Ok = (), Err = Never> + Send + 'static,
{
    //by default we log for info
    let builder = slog_envlogger::LogBuilder::new(drain).filter(None, slog::FilterLevel::Info);
    let builder = if let Ok(s) = env::var("RUST_LOG") {
        builder.parse(&s)
    } else {
        builder
    };
    let drain = slog_async::Async::new(builder.build())
        .chan_size(256)
        .build();

    let log = slog::Logger::root(drain.fuse(), slog_o!());
    let scope_guard = slog_scope::set_global_logger(log);
    slog_stdlog::init().unwrap();
    (scope_guard, ())
}
