// // while I like this macro, it messes with rustfmt
// macro_rules! pub_use_mod {
//     ($mod:ident) => {
//         pub mod $mod;
//         pub use $mod::*;
//     };
// }

// pub_use_mod!(error);
// pub_use_mod!(id);
// pub_use_mod!(message);
// pub_use_mod!(peer);
// pub_use_mod!(peerlist);

pub mod error;
pub use error::*;
pub mod id;
pub use id::*;
pub mod message;
pub use message::*;
pub mod peer;
pub use peer::*;
pub mod peerlist;
pub use peerlist::*;

pub mod util;
// pub use util::parse_ipv4socket;
