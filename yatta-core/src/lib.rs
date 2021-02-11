use anyhow::Result;
use clap::Clap;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display)]
pub enum SocketMessage {
    FocusWindow(OperationDirection),
    MoveWindow(OperationDirection),
    Promote,
    TogglePause,
    Retile,
    SetGapSize(i32),
    AdjustGaps(Sizing),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[derive(Clap)]
pub enum OperationDirection {
    Left,
    Right,
    Up,
    Down,
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[derive(Clap)]
pub enum Sizing {
    Increase,
    Decrease,
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
