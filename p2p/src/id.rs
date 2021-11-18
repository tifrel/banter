use crate::{util, Error};
use bytes::{Buf, BufMut, BytesMut};
use std::{
    convert::{AsRef, From, Into},
    fmt,
    str::FromStr,
};

/// Holds a unique ID for each Peer. Designed to be drawn at random, as there
/// 2.23e43 (256 ** 18) possible addresses.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct PeerId([u8; 18]);

impl PeerId {
    /// Generates a new empty peer ID.
    #[inline]
    pub fn empty() -> Self {
        Self([0; 18])
    }

    /// Generates a new random peer ID. This has to initialize a RNG internally,
    /// so for generation of many IDs, `PeerId::by_rng` is preferable.
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        Self::by_rng(&mut rng)
    }

    /// Generates a new random peer ID from an externally initialized RNG.
    pub fn by_rng<R: rand::Rng>(rng: &mut R) -> Self {
        let mut id = [0; 18];
        rng.fill_bytes(&mut id);
        PeerId(id)
    }

    /// Reads a peer ID from a `bytes::BytesMut` buffer.
    pub fn read_from_bytes(bytes: &mut BytesMut) -> Result<Self, Error> {
        if bytes.remaining() < 18 {
            Err(Error::Deserialization)
        } else {
            let mut id = [0; 18];
            bytes.copy_to_slice(&mut id);
            Ok(Self(id))
        }
    }

    /// Writes a peer ID to a `bytes::BytesMut` buffer.
    pub fn write_to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_slice(&self.0)
    }
}

impl AsRef<[u8; 18]> for PeerId {
    #[inline]
    fn as_ref(&self) -> &[u8; 18] {
        &self.0
    }
}

impl Into<[u8; 18]> for PeerId {
    #[inline]
    fn into(self) -> [u8; 18] {
        self.0
    }
}

impl From<[u8; 18]> for PeerId {
    #[inline]
    fn from(bytes: [u8; 18]) -> Self {
        Self(bytes)
    }
}

impl FromStr for PeerId {
    type Err = Error;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let id_vec = util::base64_decode(string)?;
        if id_vec.len() != 18 {
            Err(Error::Parsing {
                from: string.to_string(),
                to: "PeerId".to_string(),
            })
        } else {
            let mut id = [0; 18];
            id.copy_from_slice(&id_vec[..]);
            Ok(Self(id))
        }
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &util::base64_encode(self.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peer_id_serde() {
        let ids = [
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
        ];

        let mut bytes = BytesMut::new();
        for id in ids {
            id.write_to_bytes(&mut bytes);
            assert_eq!(id, PeerId::read_from_bytes(&mut bytes).unwrap());
        }
    }
}
