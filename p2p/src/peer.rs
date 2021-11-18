use crate::{util, Error, PeerId};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::{
    fmt,
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};
use tokio::{io::AsyncWriteExt, net::TcpStream, time};

/// Struct to identify peers and send messages:
/// ```
/// let peer = banter::Peer::try_from_socket_str("127.0.0.1:12001").unwrap();
/// let mut bytes = bytes::Bytes::from(b"Hello, World!");
///
/// peer.send(bytes).await;
/// ```
/// (Note that this doctest fails because it requires to setup an async runtime
/// and a peer, which is not yet implemented)
///
/// This also keeps track of the last heartbeat to determine if the peer
/// has been online recently enough to keep in memory.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Peer {
    pub id: PeerId,
    pub addr: SocketAddrV4,
    pub last_heartbeat: Option<time::Instant>,
}

impl Peer {
    /// Create peer from known ID and address.
    #[inline]
    pub fn new(id: PeerId, addr: SocketAddrV4) -> Self {
        Self {
            id,
            addr,
            last_heartbeat: None,
        }
    }

    /// Create peer from address with random ID. Mainly used to initialize the
    /// using node as a peer itself.
    #[inline]
    pub fn with_random_id(addr: SocketAddrV4) -> Self {
        Self::new(PeerId::random(), addr)
    }

    /// Create peer from IPv4 octets and port number.
    pub fn from_socket(id: PeerId, ip: [u8; 4], port: u16) -> Self {
        let ip = Ipv4Addr::from(ip);
        let addr = SocketAddrV4::new(ip, port);
        Self::new(id, addr)
    }

    /// Create peer from IPv4 octets and port number.
    pub fn from_socket_random_id(ip: [u8; 4], port: u16) -> Self {
        let ip = Ipv4Addr::from(ip);
        let addr = SocketAddrV4::new(ip, port);
        Self::new(PeerId::random(), addr)
    }

    /// Try to create peer from ID, IPv4, and port number, given as a string.
    #[inline]
    pub fn try_from_socket_str(
        id: PeerId,
        socket: &str,
    ) -> Result<Self, Error> {
        util::parse_ipv4socket(socket).map(|addr| Self::new(id, addr))
    }

    /// Try to create peer from IPv4 and port number, given as string. An empty,
    /// all-zero peer ID will be used. Intended to connect to a peer with known
    /// socket address but unknown ID.
    #[inline]
    pub fn try_from_socket_str_empty_id(socket: &str) -> Result<Self, Error> {
        util::parse_ipv4socket(socket)
            .map(|addr| Self::new(PeerId::empty(), addr))
    }

    // (maybe) TODO: find(id: PeerId, &mut peerlist)

    /// Send buffer to peer.
    pub async fn send(&self, bytes: Bytes) -> Result<(), Error> {
        let mut stream = TcpStream::connect(self.addr).await.map_err(|_| {
            Error::ConnectionFailed {
                to: format!("{}", self.addr),
            }
        })?;
        stream.write(bytes.as_ref()).await.map_err(|_| {
            Error::SocketWriteFailed {
                to: format!("{}", self.addr),
            }
        })?;
        let _ = stream.flush().await.map_err(|_| Error::SocketWriteFailed {
            to: format!("{}", self.addr),
        })?;
        Ok(())
    }

    /// Write ID and socket address into bytes buffer.
    pub fn write_to_bytes(&self, bytes: &mut BytesMut) {
        self.id.write_to_bytes(bytes);
        bytes.put_slice(&self.addr.ip().octets());
        bytes.put_u16(self.addr.port());
    }

    /// Read ID and socket address from bytes buffer.
    pub fn read_from_bytes(bytes: &mut BytesMut) -> Result<Self, Error> {
        let id = PeerId::read_from_bytes(bytes)?;
        if bytes.remaining() < 6 {
            Err(Error::Deserialization)
        } else {
            let addr = {
                let mut ip = [0; 4];
                ip[0] = bytes.get_u8();
                ip[1] = bytes.get_u8();
                ip[2] = bytes.get_u8();
                ip[3] = bytes.get_u8();
                let port = bytes.get_u16();
                SocketAddrV4::new(Ipv4Addr::from(ip), port)
            };
            Ok(Peer::new(id, addr))
        }
    }

    /// Format ID and socket address as string.
    #[inline]
    pub fn to_string(&self) -> String {
        format!("{}::{}", self.id, self.addr)
    }
}

impl FromStr for Peer {
    type Err = Error;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (id, addr) = string.split_once("::").ok_or(Error::Parsing {
            from: string.to_string(),
            to: "Peer".to_string(),
        })?;

        let id = id.parse()?;
        let addr = util::parse_ipv4socket(addr)?;
        Ok(Self {
            id,
            addr,
            last_heartbeat: None,
        })
    }
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_serde() {
        let peers = [
            Peer::from_socket(PeerId::random(), [127, 0, 0, 1], 32_000),
            Peer::from_socket(PeerId::random(), [192, 168, 0, 25], 60_000),
            Peer::from_socket(PeerId::random(), [255, 255, 255, 0], 1),
            Peer::from_socket(PeerId::random(), [3, 141, 59, 26], 53589),
        ];

        let mut bytes = BytesMut::new();
        for peer in peers {
            peer.write_to_bytes(&mut bytes);
            assert_eq!(peer, Peer::read_from_bytes(&mut bytes).unwrap());
        }
    }
}
