#[macro_use]
extern crate clap;

use banter::*;
use tokio::io::AsyncReadExt;

// TODO: read config from file, write when exiting
// when exiting, can be sent via quit_rx

fn init() -> NodeOptions {
    let app = clap::clap_app!(app =>
        (version: "0.1.0")
        (author: "Till Friesewinkel <till.friesewinkel@gmail.com>")
        (about: "CLI peer exploration for the banter protocol")
        (@arg port: -p --port +takes_value "Specifies the port to use")
        (@arg peers: -P --peers +takes_value "Specifies the initial peers to contact")
        (@arg id: -i --id +takes_value "Specifies the ID for this node")
        // (@arg id: -s --serial +takes_value "Specifies initial serial")
        // (@arg id: -c --cfg +takes_value "Specifies a config file")
    )
    .get_matches();

    let port: u16 = app.value_of("port").map_or(12000, |s| s.parse().unwrap());
    let id: PeerId = app
        .value_of("id")
        .map_or(PeerId::random(), |s| s.parse().unwrap());
    let init_peers = app
        .value_of("peers")
        .map_or(vec![], |s| s.split(',').map(|s| s.to_string()).collect());

    NodeOptions {
        port,
        id,
        init_peers,
    }
}

#[tokio::main]
async fn main() {
    let opts = init();
    let (mut node, stdin_tx, mut stdout_rx, mut stderr_rx, mut quit_rx) =
        Node::init(opts).unwrap();

    // spawn task for running the node
    tokio::spawn(async move {
        node.run().await;
    });

    println!("Good to go, let's start bantering!");

    // IO handling
    let mut stdin = tokio::io::stdin();
    let mut buffer = [0; 16384];
    'main: loop {
        // print!("> ");
        tokio::select! {
            Ok(n) = stdin.read(&mut buffer) => {
                let input = String::from_utf8(buffer[..n].to_vec()).unwrap();
                stdin_tx.send(input.trim().into()).await.unwrap();
            },

            Some(msg) = stdout_rx.recv() => {
                println!("\r{}", msg)
            },

            Some(err) = stderr_rx.recv() => {
                print!("\r");
                eprintln!("{}", err)
            },

            Some(()) = quit_rx.recv() => {
                print!("\r");
                break 'main;
            }
        }
    }

    // explicit exit required (otherwise stdin blocks exiting)
    std::process::exit(0);
}
