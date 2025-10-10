use std::fmt::{Display, Formatter};
use vk_macros::Vector;

#[derive(Clone, Copy, Default)]
pub enum KeyTypeExtended {
    #[default]
    All,
    None,
    KeyType(KeyType),
}

impl KeyTypeExtended {
    pub fn vector() -> Vec<Self> {
        let mut vec = vec![KeyTypeExtended::All];
        vec.extend(KeyType::vector().into_iter().map(KeyTypeExtended::KeyType));
        vec
    }
}

impl Display for KeyTypeExtended {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyTypeExtended::All => write!(f, "All Key Types"),
            KeyTypeExtended::None => write!(f, "Unknown"),
            KeyTypeExtended::KeyType(kt) => kt.fmt(f),
        }
    }
}

impl KeyTypeExtended {
    pub fn to_resp_str(&self) -> &'static str {
        match self {
            KeyTypeExtended::All => "",
            KeyTypeExtended::None => "",
            KeyTypeExtended::KeyType(kt) => kt.to_resp_str(),
        }
    }
}
impl<T: AsRef<str>> From<T> for KeyTypeExtended {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "hash" => KeyTypeExtended::KeyType(KeyType::Hash),
            "list" => KeyTypeExtended::KeyType(KeyType::List),
            "set" => KeyTypeExtended::KeyType(KeyType::Set),
            "zset" => KeyTypeExtended::KeyType(KeyType::SortedSet),
            "string" => KeyTypeExtended::KeyType(KeyType::String),
            "bloomfltr" => KeyTypeExtended::KeyType(KeyType::Bloom),
            _ => KeyTypeExtended::None,
        }
    }
}

impl From<KeyType> for KeyTypeExtended {
    fn from(kt: KeyType) -> Self {
        KeyTypeExtended::KeyType(kt)
    }
}

#[derive(Clone, Copy, Default, Vector)]
pub enum KeyType {
    #[default]
    Hash,
    List,
    Set,
    SortedSet,
    String,
    Bloom,
}

impl KeyType {
    pub fn to_resp_str(&self) -> &'static str {
        match self {
            KeyType::Hash => "TYPE hash",
            KeyType::List => "TYPE list",
            KeyType::Set => "TYPE set",
            KeyType::SortedSet => "TYPE zset",
            KeyType::String => "TYPE string",
            KeyType::Bloom => "TYPE bloomfltr",
        }
    }
}

impl<T: AsRef<str>> From<T> for KeyType {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "hash" => KeyType::Hash,
            "list" => KeyType::List,
            "set" => KeyType::Set,
            "zset" => KeyType::SortedSet,
            "string" => KeyType::String,
            "bloomfltr" => KeyType::Bloom,
            _ => KeyType::Hash,
        }
    }
}

impl From<KeyTypeExtended> for KeyType {
    fn from(kte: KeyTypeExtended) -> Self {
        match kte {
            KeyTypeExtended::All => KeyType::Hash,
            KeyTypeExtended::None => KeyType::Hash,
            KeyTypeExtended::KeyType(kt) => kt,
        }
    }
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::Hash => write!(f, "Hash"),
            KeyType::List => write!(f, "List"),
            KeyType::Set => write!(f, "Set"),
            KeyType::SortedSet => write!(f, "Sorted Set"),
            KeyType::String => write!(f, "String"),
            KeyType::Bloom => write!(f, "Bloomfilter"),
        }
    }
}
