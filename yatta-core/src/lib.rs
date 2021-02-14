use anyhow::Result;
use clap::Clap;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display)]
pub enum SocketMessage {
    AdjustGaps(Sizing),
    FocusWindow(OperationDirection),
    MoveWindow(OperationDirection),
    Promote,
    Retile,
    Layout(Layout),
    CycleLayout(CycleDirection),
    GapSize(i32),
    ToggleFloat,
    TogglePause,
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
pub enum Layout {
    BSPV,
    BSPH,
    Columns,
    Rows,
    Monocle,
}

impl Layout {
    pub fn next(&mut self) {
        match self {
            Layout::BSPV => *self = Layout::BSPH,
            Layout::BSPH => *self = Layout::Columns,
            Layout::Columns => *self = Layout::Rows,
            Layout::Rows => *self = Layout::Monocle,
            Layout::Monocle => *self = Layout::BSPV,
        }
    }

    pub fn previous(&mut self) {
        match self {
            Layout::BSPV => *self = Layout::Monocle,
            Layout::BSPH => *self = Layout::BSPV,
            Layout::Columns => *self = Layout::BSPH,
            Layout::Rows => *self = Layout::Columns,
            Layout::Monocle => *self = Layout::Rows,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[derive(Clap)]
pub enum CycleDirection {
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
