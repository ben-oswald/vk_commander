mod key_type;
pub mod valkey_client;
mod valkey_url;
mod valkey_value;

pub use key_type::{KeyType, KeyTypeExtended};
pub use valkey_url::{ValkeyUrl, ValkeyUrlBuilder};
pub use valkey_value::ValkeyValue;

pub trait Len {
    fn len(&self) -> usize;
}

pub trait ToResp {
    fn to_resp(&self) -> String;
}

pub trait ToVec {
    fn to_vec(&self) -> Vec<String>;
}

fn find_crlf(data: &[u8], start: usize) -> Option<usize> {
    (start..data.len().saturating_sub(1)).find(|&i| data[i] == b'\r' && data[i + 1] == b'\n')
}
