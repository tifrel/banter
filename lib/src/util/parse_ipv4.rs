use crate::Error;
use std::net::{Ipv4Addr, SocketAddrV4};

pub fn parse_ipv4socket(s: &str) -> Result<SocketAddrV4, Error> {
    macro_rules! parse_ipv4_error {
        ($s:expr) => {
            Error::Parsing {
                from: $s.to_string(),
                to: "SocketAddrV4".to_string(),
            }
        };
    }

    macro_rules! pop_byte_from_ip_str {
        ($s:expr) => {{
            let tmp = $s.split_once('.').ok_or(parse_ipv4_error!(s))?;
            $s = tmp.1;
            parse_byte_from_str!(tmp.0)
        }};
    }

    macro_rules! parse_byte_from_str {
        ($s:expr) => {
            $s.parse().map_err(|_| parse_ipv4_error!(s))
        };
    }

    let (mut ip_s, port_s) = s.split_once(':').ok_or(parse_ipv4_error!(s))?;
    let port: u16 = port_s.parse().map_err(|_| parse_ipv4_error!(s))?;
    let mut octets = [0; 4];
    octets[0] = pop_byte_from_ip_str!(ip_s)?;
    octets[1] = pop_byte_from_ip_str!(ip_s)?;
    octets[2] = pop_byte_from_ip_str!(ip_s)?;
    octets[3] = parse_byte_from_str!(ip_s)?;

    if !ip_s.contains(".") {
        Ok(SocketAddrV4::new(Ipv4Addr::from(octets), port))
    } else {
        Err(parse_ipv4_error!(s))
    }
}
