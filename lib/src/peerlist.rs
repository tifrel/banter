use crate::{Error, P2pMsg, Peer, PEER_BYTES_LEN};
use bytes::{Bytes, BytesMut, BufMut};
use std::{collections::HashMap, convert::{Into, TryFrom}, sync::{Arc, Mutex}};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time,
};

#[derive(Debug)]
pub struct Peerlist {
    task: tokio::task::JoinHandle<()>,
    peers: Arc<Mutex<[Option<Peer>; 128]>>,
}

/// Options required for creating a peerlist
pub struct PeerlistOptions {
    /// The socket address (IPv4 and port) that this node should bind to.
    pub my_addr: String,
    /// Peers that the list will try to contact upon initialization.
    /// To connect to any network, at least one of these peers is required to
    /// respond.
    pub init_peers: Vec<String>,
    /// The minimum duration between heartbeats. If a heartbeat is triggered
    /// before the minimum duration has passed since the last heartbeat (this
    /// happens e.g. by a heartbeat received from the network), no heartbeat
    /// will be emitted. This is to prevent the network from clogging with
    /// heartbeats.
    pub heartbeat_min: time::Duration,
    /// Used to set an internal intervalled trigger to broadcast heartbeats to
    /// the network
    pub heartbeat_avg: time::Duration,
    /// If no heartbeat has been received in this duration, a peer will be
    /// removed form the list of current peers
    pub heartbeat_max: time::Duration,
    /// The serial number with which this nodes data messages begin. Every node
    /// keeps track of the serials to prevent re-broadcasting messages. If you
    /// reconnect to a network, make sure that your initial serial number is
    /// higher than that of the last data you sent.
    pub init_serial: u64,
    /// Sife of the internal buffer to read messages. As of now, messages that
    /// exceed this size will cause a panic.
    pub buffer_size: usize
}

impl Peerlist {
    /// Create a new Peerlist that handles connections to the network.
    pub fn new(opts: PeerlistOptions) -> Result<(Self, mpsc::Sender<Bytes>, mpsc::Receiver<Bytes>), Error> {
        let (tx_external, rx_external) = mpsc::channel(128);
        let (tx_internal, mut rx_internal) = mpsc::channel(128);
        let self_peer = Peer::try_from_socket_str(&opts.my_addr)?;
        let peers = Arc::new(Mutex::new([None; 128]));
        let task_peers = peers.clone();

        let task = tokio::spawn(async move {
            let listener = TcpListener::bind(&opts.my_addr).await.unwrap();
            println!("Listening on TCP socket");

            // initial heartbeat + request peerlist
            for peer in opts.init_peers.iter() {
                let peer = Peer::try_from_socket_str(peer).unwrap();
                let _ = peer.send(P2pMsg::Heartbeat(self_peer).into()).await;
                let _ = peer.send(P2pMsg::RequestPeerlist(self_peer).into()).await;
            }

            let mut state = PeerlistState::new(task_peers, self_peer, tx_external, opts);

            loop {
                let sleep = time::sleep_until(state.next_heartbeat);
                tokio::select! {
                    connection = listener.accept() => {
                        if let Ok((socket, _)) = connection {
                            state.handle_connection(socket).await;
                        }
                    }
                    _ = sleep => {
                        state.broadcast_heartbeat().await;
                    }
                    Some(data) = rx_internal.recv() => {
                        state.broadcast_data(data).await;
                    }
                }
            }
        });

        Ok((Self { task, peers }, tx_internal, rx_external))
    }

    pub fn get_peers(&self) -> Peers {
        Peers {list: self.peers.clone(), i: 0}
    }
}

// Holds the state and is used in (and thus moved into) the async block.
// Only the actual list is shared with
// TODO: To allow logging in with my "credentials", we need to store they id,
// and the serials. Storing peers might be a healthy idea for reconnecting.
struct PeerlistState {
    peer: Peer,
    serial: u64,
    peers: Arc<Mutex<[Option<Peer>; 128]>>,
    serials: HashMap<Peer, u64>,
    buffer: BytesMut,
    last_heartbeat: time::Instant,
    next_heartbeat: time::Instant,
    heartbeat_min: time::Duration,
    heartbeat_avg: time::Duration,
    heartbeat_max: time::Duration,
    tx_external: mpsc::Sender<Bytes>,
}

impl PeerlistState {
    fn new(peers: Arc<Mutex<[Option<Peer>; 128]>>, self_peer: Peer, tx_external: mpsc::Sender<Bytes>, opts: PeerlistOptions) -> Self {
        let mut buffer = BytesMut::with_capacity(opts.buffer_size);
        buffer.resize(opts.buffer_size, 0);
        // let heartbeat_min = time::Duration::from_millis(opts.heartbeat_min);
        // let heartbeat_max = time::Duration::from_millis(opts.heartbeat_avg);
        let last_heartbeat = time::Instant::now();
        let next_heartbeat = last_heartbeat + opts.heartbeat_avg;

        Self {
            peer: self_peer,
            serial: opts.init_serial,
            peers,
            serials: HashMap::new(),
            buffer,
            last_heartbeat,
            next_heartbeat,
            heartbeat_min: opts.heartbeat_min,
            heartbeat_avg: opts.heartbeat_avg,
            heartbeat_max: opts.heartbeat_max,
            tx_external,
        }
    }

    fn as_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::with_capacity(128 * PEER_BYTES_LEN);
        for peer in self.peers() {
            peer.write_to_bytes(&mut bytes);
        }
        bytes.into()
    }

    fn insert(&mut self, peer: Peer) {
        let i = peer.id.as_ref()[17] % 128;
        let mut peers = self.peers.lock().unwrap();
        peers[i as usize] = Some(peer);
    }

    fn peers(&self) -> Peers {
        Peers { list: self.peers.clone(), i: 0 }
    }

    // --------------------- impls for messages from peers ---------------------

    // // Capped by the buffer
    // async fn read_socket(&mut self, mut socket: TcpStream) -> Option<(usize, BytesMut)> {
    //     match socket.read(&mut self.buffer).await {
    //         Ok(n) => Some((n, BytesMut::from(&self.buffer[..n]))),
    //         Err(_) => None,
    //     }
    // }

    // // Reading into vec
    // async fn read_socket(&mut self, mut socket: TcpStream) -> Option<(usize, BytesMut)> {
    //     let mut total = 0;
    //     let mut bytes = BytesMut::new();
    //     loop {
    //         match socket.read(&mut self.buffer).await {
    //             Ok(n) => {
    //                 bytes.resize(n, 0);
    //                 bytes.copy_from_slice(&mut self.buffer[..n]);
    //                 total += n;
    //                 if n == 0 {
    //                     break;
    //                 }
    //             }
    //             Err(_) => return None,
    //         }
    //     }
    //     Some((total, bytes))
    // }

    async fn handle_connection(&mut self, mut socket: TcpStream) {
        // TODO: use vec instead of buffer?
        let (n, buffer) = match socket.read(&mut self.buffer).await {
            Ok(n) => (n, BytesMut::from(&self.buffer[..n])),
            Err(_) => return,
        };

        if let Ok(msg) = P2pMsg::try_from(buffer) {
            use P2pMsg::*;
            match msg {
                Heartbeat(peer) if n == 1 + PEER_BYTES_LEN => self.handle_heartbeat(peer).await,
                RequestHeartbeat(from, serial, to) if n == 1 + PEER_BYTES_LEN => {
                    self.handle_heartbeat_request(from, serial, to).await
                }
                RequestPeerlist(peer) if n == 1 + PEER_BYTES_LEN => {
                    self.handle_peerlist_request(peer).await
                }
                Peerlist(peers) => self.handle_peerlist(peers).await,
                Data(peer, serial, data) => self.handle_data(peer, serial, data).await,
                _ => {}
            }
        }
    }

    async fn handle_heartbeat_request(&mut self, from: Peer, serial: u64,to: Peer) {
        if to == self.peer {
            let _ = from.send(P2pMsg::Heartbeat(self.peer).into()).await;
        } else if serial > *self.serials.entry(from).or_insert(0) {
            self.broadcast_bytes(P2pMsg::RequestHeartbeat(from, serial, to).into()).await;
        }
    }

    async fn handle_heartbeat(&mut self, mut peer: Peer) {
        peer.last_heartbeat = Some(time::Instant::now());
        self.insert(peer);
        self.broadcast_heartbeat().await;
    }

    async fn handle_peerlist_request(&mut self, peer: Peer) {
        let _ = peer.send(self.as_bytes()).await;
    }

    async fn handle_peerlist(&mut self, peers: Vec<Peer>) {
        for peer in peers {
            self.insert(peer);
        }
    }

    async fn handle_data(&mut self, peer: Peer, serial: u64, data: Bytes) {
        if serial > *self.serials.entry(peer).or_insert(0) {
            self.serials.insert(peer, serial);
            self.tx_external.send(data).await.unwrap();
        }
    }

    // ------------------- impls for messages from this node -------------------
    async fn broadcast_heartbeat(&mut self) {
        // To prevent clogging the network, we do not send  heartbeats unless
        // at least heartbeat_min has elapsed since the last heartbeat
        if time::Instant::now() > self.last_heartbeat + self.heartbeat_min {
            self.broadcast_bytes(P2pMsg::Heartbeat(self.peer).into())
                .await;
            self.last_heartbeat = time::Instant::now();
            self.next_heartbeat = self.last_heartbeat + self.heartbeat_avg;

            // Purge peers that went offline
            let mut peers = self.peers.lock().unwrap();
            let criterion = time::Instant::now() - self.heartbeat_max;
            for (i, peer) in peers.clone().iter().enumerate() {
                if let Some(Peer {id: _, addr: _, last_heartbeat: Some(last)}) = peer {
                    if *last < criterion  {
                        peers[i] = None;
                    }
                }
            }
        }
    }

    async fn broadcast_bytes(&self, bytes: Bytes) {
        for peer in self.peers() {
            let _ = peer.send(bytes.clone()).await;
        }
    }

    async fn broadcast_data(&mut self, bytes: Bytes) {
        let mut buffer = BytesMut::new();
        buffer.put_u64(self.serial);
        self.serial += 1;
        buffer.copy_from_slice(bytes.as_ref());
        self.broadcast_bytes(buffer.into()).await
    }
}

/// Iterator over the peers in a Peerlist
pub struct Peers {
    list: Arc<Mutex<[Option<Peer>; 128]>>,
    i: usize,
}

impl Iterator for Peers {
    type Item = Peer;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i > 127 {
            return None;
        }

        let peers = self.list.lock().unwrap();
        for peer in peers[self.i..].iter() {
            self.i += 1;
            if let Some(peer) = peer {
                return Some(*peer);
            }
        }

        None
    }
}
