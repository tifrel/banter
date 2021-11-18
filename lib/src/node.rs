use banter_p2p::{Peer, PeerId, Peerlist, PeerlistOptions};
use bytes::Bytes;
use tokio::sync::mpsc;

/// Options required to run a Banter node
pub struct NodeOptions {
    /// Port to use
    pub port: u16,
    /// The ID that this node will use.
    pub id: PeerId,
    /// Peers that the node will try to contact upon initialization.
    /// To connect to any network, at least one of these peers is required to
    /// respond.
    pub init_peers: Vec<String>,
}

/// Handle for the running node that communicates with the Banter network.
/// It handles translation of CLI IO (Strings) to Messages for the network.
pub struct Node {
    id: PeerId,
    peers: Peerlist,
    p2p_tx: mpsc::Sender<Bytes>,
    p2p_rx: mpsc::Receiver<(Peer, Bytes)>,
    stdin_rx: mpsc::Receiver<String>,
    stdout_tx: mpsc::Sender<String>,
    stderr_tx: mpsc::Sender<String>,
    quit_tx: mpsc::Sender<()>,
}

impl Node {
    /// Initializes a Banter chatting node. And begins running the p2p layer.
    ///
    /// If successful, it returns:
    ///
    ///	- The node handle itself, which starts translation once `Node::run` is
    ///   called.
    /// - A sender for strings, which will be translated into either commands or
    ///   messages to broadcast to the network.
    /// - A Receiver for messages from the network.
    /// - A Receiver for error messages.
    /// - A Receiver for a quit signal to exit the client.
    pub fn init(
        opts: NodeOptions,
    ) -> Result<
        (
            Node,
            mpsc::Sender<String>,
            mpsc::Receiver<String>,
            mpsc::Receiver<String>,
            mpsc::Receiver<()>,
        ),
        banter_p2p::Error,
    > {
        let id = opts.id; // copy required because opts is moved into peerlist
        let opts = PeerlistOptions {
            addr: format!("127.0.0.1:{}", opts.port),
            id: opts.id,
            init_peers: opts.init_peers,
            init_serial: 0,
            heartbeat_min: tokio::time::Duration::from_millis(1800),
            heartbeat_avg: tokio::time::Duration::from_millis(2000),
            heartbeat_max: tokio::time::Duration::from_millis(2200),
            buffer_size: 16384,
        };

        let (peers, p2p_tx, p2p_rx) = Peerlist::new(opts)?;
        let (stdin_tx, stdin_rx) = mpsc::channel(16);
        let (stdout_tx, stdout_rx) = mpsc::channel(16);
        let (stderr_tx, stderr_rx) = mpsc::channel(16);
        let (quit_tx, quit_rx) = mpsc::channel(1);
        Ok((
            Node {
                id,
                peers,
                p2p_tx,
                p2p_rx,
                stdin_rx,
                stdout_tx,
                stderr_tx,
                quit_tx,
            },
            stdin_tx,
            stdout_rx,
            stderr_rx,
            quit_rx,
        ))
    }

    /// While the p2p layer starts running when the node is initialized, this
    /// starts the translation between strings and commands/network messages.
    pub async fn run(&mut self) {
        'run: loop {
            tokio::select! {
                Some((peer, bytes)) = self.p2p_rx.recv() => {
                    if peer.id != self.id {
                        match String::from_utf8(bytes.as_ref().to_vec()) {
                            Ok(msg) => {
                                let msg = format!("{}: {}", peer.id, msg);
                                self.stdout_tx.send(msg).await.unwrap();
                            },

                            // Nothing, we don't bother ourselves with rubbish
                            Err(_) => {},
                        };
                    }
                }
                Some(string) = self.stdin_rx.recv() => {
                    // case A: we got a command
                    if &string[..1] == ":" {
                        match translate_cmd(&string[1..].trim_end()) {
                            Some(Cmd::Quit) => { break 'run; },
                            Some(Cmd::ListPeers) => {
                                self.list_peers().await;
                            },
                            None => {
                                self.stderr_tx.send(
                                    format!("Unknown command `{}`", string)
                                ).await.unwrap();
                            },
                        }
                    // case B: we got a message to broadcast
                    } else {
                        self.p2p_tx.send(string.into()).await.unwrap();
                    }
                }
            }
        }
        // signal quit to main loop
        self.quit_tx.send(()).await.unwrap();
    }

    async fn list_peers(&mut self) {
        // prealloc: 128 (max peers) * (21 (max socketaddr length) + 24 (ID))
        let mut string = String::with_capacity(5760);
        for peer in self.peers.get_peers() {
            string.push_str(&peer.to_string());
            string.push('\n');
        }
        self.stdout_tx.send(string).await.unwrap();
    }
}

enum Cmd {
    ListPeers,
    Quit,
    // (maybe) TODO: RequestList(id)
    // pro: current method of peer propagation might yield biased lists
    // con: more user overhead
    // solution -> rng when selecting peers to keep
    // better solution: just use libp2p
}

fn translate_cmd(cmd: &str) -> Option<Cmd> {
    match cmd {
        "q" | "quit" => Some(Cmd::Quit),
        "l" | "list" => Some(Cmd::ListPeers),
        _ => None,
    }
}
