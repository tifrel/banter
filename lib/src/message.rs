use crate::{Error, Peer};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::convert::{Into, TryFrom};

// const ID_BYTES_LEN: usize = 18;
// const ADDR_BYTES_LEN: usize = 6;
pub const PEER_BYTES_LEN: usize = 24;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum P2pMsg {
    /// Send heartbeat (and my own ID + addr) to signal that I am online.
    Heartbeat(Peer),
    /// Request a heartbeat from another peer, e.g. to find establish a direct
    /// connection. The first field is the sender of the message, the second
    /// a serial for this requeslt, and the third the peer whose heartbeat is
    /// requested.
    RequestHeartbeat (Peer, u64, Peer),
    /// Request a peerlist to update mine, used while building the initial
    /// network connection.
    RequestPeerlist(Peer),
    /// Answer to a peerlist request.
    Peerlist(Vec<Peer>),
    /// Sending data. First field is the peer who sent the data, second field a
    /// serial to avoid re-broadcasting the same message, third field the
    /// actual message.
    Data(Peer, u64, Bytes),
}

impl Into<BytesMut> for P2pMsg {
    fn into(self) -> BytesMut {
        use P2pMsg::*;
        let mut buffer = BytesMut::with_capacity(1);
        match self {
            Heartbeat(peer) => {
                buffer.put_u8(1);
                peer.write_to_bytes(&mut buffer);
            }
            RequestHeartbeat(from, serial, to) => {
            buffer.put_u8(2);
                from.write_to_bytes(&mut buffer);
                buffer.put_u64(serial);
                to.write_to_bytes(&mut buffer);
            }
            RequestPeerlist(peer) => {
                buffer.put_u8(3);
                peer.write_to_bytes(&mut buffer);
            }
            Peerlist(peers) => {
                buffer.put_u8(4);
                for peer in peers {
                    peer.write_to_bytes(&mut buffer);
                }
            }
            Data(peer, serial, data) => {
                buffer.put_u8(5);
                peer.write_to_bytes(&mut buffer);
                buffer.put_u64(serial);
                buffer.extend_from_slice(data.as_ref());
            }
        }
        buffer
    }
}

impl Into<Bytes> for P2pMsg {
    fn into(self) -> Bytes {
        <Self as Into<BytesMut>>::into(self).into()
    }
}

impl TryFrom<BytesMut> for P2pMsg {
    type Error = crate::Error;
    fn try_from(mut bytes: BytesMut) -> Result<Self, Error> {
        use P2pMsg::*;
        let type_byte = bytes.get_u8();
        let remaining = bytes.remaining();
        match type_byte {
            1 if remaining == PEER_BYTES_LEN => {
                Peer::read_from_bytes(&mut bytes).map(|peer| Heartbeat(peer))
            }
            2 if remaining == PEER_BYTES_LEN => {
                let from = Peer::read_from_bytes(&mut bytes)?;
                let serial = bytes.get_u64();
                let to = Peer::read_from_bytes(&mut bytes)?;
                Ok(Self::RequestHeartbeat(from, serial, to))
            }
            3 if remaining == PEER_BYTES_LEN => {
                Peer::read_from_bytes(&mut bytes).map(|peer| RequestPeerlist(peer))
            }
            4 if remaining % PEER_BYTES_LEN == 0 => {
                let mut peers = Vec::new();
                while bytes.remaining() > 0 {
                    peers.push(Peer::read_from_bytes(&mut bytes)?);
                }
                Ok(Peerlist(peers))
            }
            5 if remaining > PEER_BYTES_LEN + 8 => {
                let peer = Peer::read_from_bytes(&mut bytes)?;
                let serial = bytes.get_u64();
                let data = bytes.into();
                Ok(Data(peer, serial, data))
            }
            _ => Err(Error::Deserialization),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use P2pMsg::*;
    #[test]
    fn p2pmsg_serde() {
        let self_peer = Peer::from_socket([127, 0, 0, 1], 12000);
        let messages = [
            Heartbeat(self_peer),
            // RequestHeartbeat(self_peer),
            RequestPeerlist(self_peer),
            Peerlist(vec![
                Peer::from_socket([127, 0, 0, 1], 12001),
                Peer::from_socket([127, 0, 0, 1], 12002),
                Peer::from_socket([127, 0, 0, 1], 12003),
            ]),
            Data(self_peer, 0, "Hello, World!".into()),
        ];

        for msg in messages {
            let bytes: BytesMut = msg.clone().into();
            assert_eq!(msg, P2pMsg::try_from(bytes).unwrap());
        }
    }
}
