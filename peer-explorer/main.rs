#[macro_use]
extern crate clap;

use banter_p2p::*;

fn init() -> PeerlistOptions {
    let app = clap::clap_app!(app =>
        (version: "0.1.0")
        (author: "Till Friesewinkel <till.friesewinkel@gmail.com>")
        (about: "CLI peer exploration for the banter protocol")
        (@arg port: -p --port +takes_value "Specifies the port to use")
        (@arg peers: -P --peers +takes_value "Specifies the initial peers to contact")
        (@arg id: -i --id +takes_value "Specifies ID to use in base64")
    )
    .get_matches();

    let port: u16 = app.value_of("port").map_or(12000, |s| s.parse().unwrap());
    let id: PeerId = app
        .value_of("id")
        .map_or(PeerId::random(), |s| s.parse().unwrap());
    let init_peers = app
        .value_of("peers")
        .map_or(vec![], |s| s.split(',').map(|s| s.to_string()).collect());

    PeerlistOptions {
        addr: format!("127.0.0.1:{}", port),
        id,
        init_peers,
        heartbeat_min: tokio::time::Duration::from_millis(1800),
        heartbeat_avg: tokio::time::Duration::from_millis(2000),
        heartbeat_max: tokio::time::Duration::from_millis(2200),
        init_serial: 0,
        buffer_size: 1024,
    }
}

#[tokio::main]
async fn main() {
    let opts = init();
    let print_interval = opts.heartbeat_avg;

    // participate in p2p network
    let (p2p, _p2p_tx, _p2p_rx) = Peerlist::new(opts).unwrap();

    // print current peerlist
    let mut interval = tokio::time::interval(print_interval);
    loop {
        interval.tick().await;
        println!("Current peers:");
        for peer in p2p.get_peers() {
            println!("{}", peer);
        }
        println!("");
    }
}
