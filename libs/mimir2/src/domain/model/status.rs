use std::fmt;

#[derive(Debug)]
pub enum StorageHealth {
    OK,
    FAIL,
}

impl fmt::Display for StorageHealth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StorageHealth::OK => write!(f, "ok"),
            StorageHealth::FAIL => write!(f, "fail"),
        }
    }
}

pub type StorageVersion = String;

#[derive(Debug)]
pub struct StorageStatus {
    pub health: StorageHealth,
    pub version: StorageVersion,
}
