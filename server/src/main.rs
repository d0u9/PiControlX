use tokio::runtime::Runtime;

// extern crate lib;
use lib::config::Config;
use clap::{Arg, App};

async fn tokio_main(config: Config) {
    lib::lib_main(config).await;
}

fn setup_opts() -> Config {
    let matches = App::new("PiControlX server")
        .version("1.0")
        .author("Douglas Su")
        .arg(Arg::with_name("config")
             .short("c")
             .long("config")
             .value_name("FILE")
             .default_value("/etc/xxx")
             .takes_value(true))
        .arg(Arg::with_name("ip")
             .long("ip")
             .value_name("Address")
             .default_value("[::1]")
             .takes_value(true))
        .arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .value_name("PORT")
             .default_value("50051")
             .takes_value(true))
        .get_matches();

    let config = matches.value_of("config").unwrap();
    let ip = matches.value_of("ip").unwrap();
    let port = matches.value_of("port").unwrap();

    Config {
        config: config.into(),
        ip: ip.into(),
        port: port.parse().unwrap(),
    }
}

fn main() {
    let config = setup_opts();
    let rt = Runtime::new().unwrap();
    rt.block_on(tokio_main(config));
}
