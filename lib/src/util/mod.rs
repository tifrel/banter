macro_rules! pub_use_mod {
    ($mod:ident) => {
        pub mod $mod;
        pub use $mod::*;
    };
}

pub_use_mod!(parse_ipv4);
pub_use_mod!(base64);
