use anyhow::Result;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display)]
pub enum SocketMessage {
    FocusWindow(OperationDirection),
    SwapWindow(OperationDirection),
    Promote,
    TogglePause,
    ReTile,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum OperationDirection {
    Previous,
    Next,
}

impl OperationDirection {
    pub fn destination(self, idx: usize, len: usize) -> usize {
        match self {
            OperationDirection::Previous => {
                if idx == 0 {
                    len - 1
                } else {
                    idx - 1
                }
            }
            OperationDirection::Next => {
                if idx == len - 1 {
                    0
                } else {
                    idx + 1
                }
            }
        }
    }
}

impl SocketMessage {
    pub fn as_bytes(self) -> Result<Vec<u8>> {
        Ok(serde_json::to_string(&self)?.as_bytes().to_vec())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }

    pub fn from_str(str: &str) -> Result<Self> {
        Ok(serde_json::from_str(str)?)
    }
}
