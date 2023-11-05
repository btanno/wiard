use windows::Win32::Foundation::{POINT, SIZE};

pub mod coord {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct LogicalCoord;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct PhysicalCoord;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct ScreenCoord;
}

use coord::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position<T, Coord> {
    pub x: T,
    pub y: T,
    #[cfg_attr(feature = "serde", serde(skip))]
    _coord: std::marker::PhantomData<Coord>,
}

impl<T, Coord> Position<T, Coord> {
    pub const fn new(x: T, y: T) -> Self {
        Self {
            x,
            y,
            _coord: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Size<T, Coord> {
    pub width: T,
    pub height: T,
    #[cfg_attr(feature = "serde", serde(skip))]
    _coord: std::marker::PhantomData<Coord>,
}

impl<T, Coord> Size<T, Coord> {
    pub const fn new(width: T, height: T) -> Self {
        Self {
            width,
            height,
            _coord: std::marker::PhantomData,
        }
    }
}

pub type LogicalPosition<T> = Position<T, LogicalCoord>;
pub type PhysicalPosition<T> = Position<T, PhysicalCoord>;
pub type ScreenPosition<T> = Position<T, ScreenCoord>;

pub type LogicalSize<T> = Size<T, LogicalCoord>;
pub type PhysicalSize<T> = Size<T, PhysicalCoord>;

impl From<POINT> for PhysicalPosition<i32> {
    #[inline]
    fn from(value: POINT) -> Self {
        PhysicalPosition::new(value.x, value.y)
    }
}

impl From<PhysicalPosition<i32>> for POINT {
    #[inline]
    fn from(value: PhysicalPosition<i32>) -> POINT {
        POINT {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<SIZE> for PhysicalSize<u32> {
    #[inline]
    fn from(value: SIZE) -> Self {
        PhysicalSize::new(value.cx as u32, value.cy as u32)
    }
}

impl From<PhysicalSize<u32>> for SIZE {
    #[inline]
    fn from(value: PhysicalSize<u32>) -> SIZE {
        SIZE {
            cx: value.width as i32,
            cy: value.height as i32,
        }
    }
}

pub const DEFAULT_DPI: u32 = 96;

pub trait ToLogical<T> {
    type Output<U>;

    fn to_logical(&self, dpi: T) -> Self::Output<T>;
}

pub trait ToPhysical<T> {
    type Output<U>;

    fn to_physical(&self, dpi: T) -> Self::Output<T>;
}

fn to_logical_value<T>(a: T, dpi: T) -> T
where
    T: num::Num + num::NumCast,
{
    a * num::cast(DEFAULT_DPI).unwrap() / dpi
}

fn to_physical_value<T>(a: T, dpi: T) -> T
where
    T: num::Num + num::NumCast,
{
    a * dpi / num::cast(DEFAULT_DPI).unwrap()
}

impl<T> ToLogical<T> for LogicalPosition<T>
where
    T: Copy,
{
    type Output<U> = LogicalPosition<U>;

    #[inline]
    fn to_logical(&self, _dpi: T) -> Self::Output<T> {
        *self
    }
}

impl<T> ToLogical<T> for PhysicalPosition<T>
where
    T: num::Num + num::NumCast + Copy,
{
    type Output<U> = LogicalPosition<U>;

    #[inline]
    fn to_logical(&self, dpi: T) -> Self::Output<T> {
        Position::new(to_logical_value(self.x, dpi), to_logical_value(self.y, dpi))
    }
}

impl<T> ToLogical<T> for LogicalSize<T>
where
    T: Copy,
{
    type Output<U> = LogicalSize<U>;

    #[inline]
    fn to_logical(&self, _dpi: T) -> Self::Output<T> {
        *self
    }
}

impl<T> ToLogical<T> for PhysicalSize<T>
where
    T: num::Num + num::NumCast + Copy,
{
    type Output<U> = LogicalSize<U>;

    #[inline]
    fn to_logical(&self, dpi: T) -> Self::Output<T> {
        Size::new(
            to_logical_value(self.width, dpi),
            to_logical_value(self.height, dpi),
        )
    }
}

impl<T> ToPhysical<T> for LogicalPosition<T>
where
    T: num::Num + num::NumCast + Copy,
{
    type Output<U> = PhysicalPosition<U>;

    #[inline]
    fn to_physical(&self, dpi: T) -> Self::Output<T> {
        Position::new(
            to_physical_value(self.x, dpi),
            to_physical_value(self.y, dpi),
        )
    }
}

impl<T> ToPhysical<T> for LogicalSize<T>
where
    T: num::Num + num::NumCast + Copy,
{
    type Output<U> = PhysicalSize<U>;

    #[inline]
    fn to_physical(&self, dpi: T) -> Self::Output<T> {
        Size::new(
            to_physical_value(self.width, dpi),
            to_physical_value(self.height, dpi),
        )
    }
}

impl<T> ToPhysical<T> for PhysicalPosition<T>
where
    T: Copy
{
    type Output<U> = PhysicalPosition<U>;

    #[inline]
    fn to_physical(&self, _dpi: T) -> Self::Output<T> {
        *self
    }
}

impl<T> ToPhysical<T> for PhysicalSize<T>
where
    T: Copy
{
    type Output<U> = PhysicalSize<U>;

    #[inline]
    fn to_physical(&self, _dpi: T) -> Self::Output<T> {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_to_logical_position() {
        let src = LogicalPosition::new(128, 256);
        let dest = src.to_logical(DEFAULT_DPI * 2);
        assert!(src == dest);
    }

    #[test]
    fn logical_to_physical_position() {
        let src = LogicalPosition::new(128, 256);
        let dest = src.to_physical(DEFAULT_DPI * 2);
        assert!(src.x * 2 == dest.x);
        assert!(src.y * 2 == dest.y);
    }

    #[test]
    fn physical_to_logical_position() {
        let src = PhysicalPosition::new(128, 256);
        let dest = src.to_logical(DEFAULT_DPI * 2);
        assert!(src.x == dest.x * 2);
        assert!(src.y == dest.y * 2);
    }

    #[test]
    fn physical_to_physical_position() {
        let src = PhysicalPosition::new(128, 256);
        let dest = src.to_physical(DEFAULT_DPI * 2);
        assert!(src == dest);
    }

    #[test]
    fn logical_to_logical_size() {
        let src = LogicalSize::new(128, 256);
        let dest = src.to_logical(DEFAULT_DPI * 2);
        assert!(src == dest);
    }

    #[test]
    fn logical_to_physical_size() {
        let src = LogicalSize::new(128, 256);
        let dest = src.to_physical(DEFAULT_DPI * 2);
        assert!(src.width * 2 == dest.width);
        assert!(src.height * 2 == dest.height);
    }

    #[test]
    fn physical_to_logical_size() {
        let src = PhysicalSize::new(128, 256);
        let dest = src.to_logical(DEFAULT_DPI * 2);
        assert!(src.width == dest.width * 2);
        assert!(src.height == dest.height * 2);
    }

    #[test]
    fn physical_to_physical_size() {
        let src = PhysicalSize::new(128, 256);
        let dest = src.to_physical(DEFAULT_DPI * 2);
        assert!(src == dest);
    }
}
