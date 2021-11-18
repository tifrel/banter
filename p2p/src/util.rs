use crate::Error;
use std::net::SocketAddrV4;

pub fn parse_ipv4socket(string: &str) -> Result<SocketAddrV4, Error> {
    string.parse().map_err(|_| Error::Parsing {
        from: string.to_string(),
        to: "SocketAddrV4".to_string(),
    })
}

const BASE64_CONFIG: base64::Config = base64::URL_SAFE_NO_PAD;

pub fn base64_encode(bytes: &[u8]) -> String {
    base64::encode_config(bytes, BASE64_CONFIG)
}

pub fn base64_decode(string: &str) -> Result<Vec<u8>, Error> {
    base64::decode_config(string, BASE64_CONFIG).map_err(|_| {
        Error::Base64Decode {
            string: string.into(),
        }
    })
}
