use log;
use simplelog::{ColorChoice, LevelFilter, TermLogger, TerminalMode};
use tokio::signal;

pub mod config;

mod caches;
mod public;
mod server;
use crate::config::Config;
use crate::public::event_queue::EventQ;
use server::server::Server;

fn setup_logger() {
    TermLogger::init(
        LevelFilter::Debug,
        // simplelog::Config::default(),
        simplelog::ConfigBuilder::new()
            .set_location_level(LevelFilter::Debug)
            .set_target_level(LevelFilter::Debug)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();
}

pub async fn lib_main(config: Config) {
    setup_logger();

    log::warn!("async main start...");

    let event_q = EventQ::new();

    let mut cache_manager = caches::CacheManager::new();
    let cache_handlers = cache_manager.create_caches(event_q.get_notifier());

    let addr = format!("{}:{}", config.ip, config.port);
    let addr = addr.parse().unwrap();
    let (mut server, server_handler) = Server::new(addr);
    server.add_caches(cache_handlers).await;

    let handler1 = tokio::spawn(async move {
        server.serve(event_q).await;
    });

    let cache_manager_handler = cache_manager.run();

    signal::ctrl_c().await.unwrap();
    tokio::join!(cache_manager_handler.shutdown(), server_handler.shutdown());
    handler1.await.unwrap();
}
