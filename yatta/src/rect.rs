use std::fmt::{Display, Error, Formatter};

use bindings::Windows::Win32::Foundation::RECT;

/// x & y coordinates are relative to top left of screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x:      i32,
    pub y:      i32,
    pub width:  i32,
    pub height: i32,
}

impl Rect {
    pub fn contains_point(self, point: (i32, i32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }

    pub fn zero() -> Self {
        Rect {
            x:      0,
            y:      0,
            width:  0,
            height: 0,
        }
    }

    pub fn adjust_for_border(&mut self, border: (i32, i32)) {
        self.x -= border.0;
        self.width += border.0 * 2;
        self.height += border.1;
    }
}

impl Display for Rect {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        writeln!(f, "x: {}", self.x)?;
        writeln!(f, "y: {}", self.y)?;
        writeln!(f, "width: {}", self.width)?;
        writeln!(f, "height: {}", self.height)?;

        Ok(())
    }
}

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Rect {
            x:      rect.left,
            y:      rect.top,
            width:  rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}
