use windows::Win32::Foundation::{POINT, RECT, SIZE};

pub mod coord {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct LogicalCoord;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct PhysicalCoord;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct ScreenCoord;
}

use coord::*;

/// The generic position object.
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

/// The generic size object.
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect<T, Coord> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T,
    #[cfg_attr(feature = "serde", serde(skip))]
    _coord: std::marker::PhantomData<Coord>,
}

impl<T, Coord> Rect<T, Coord> {
    pub const fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
            _coord: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn from_position_size(position: Position<T, Coord>, size: Size<T, Coord>) -> Self
    where
        T: std::ops::Add<T, Output = T> + Copy,
    {
        Self {
            left: position.x,
            top: position.y,
            right: position.x + size.width,
            bottom: position.y + size.height,
            _coord: std::marker::PhantomData,
        }
    }
}

/// The type of logical position.
pub type LogicalPosition<T> = Position<T, LogicalCoord>;
/// The type of physical position.
pub type PhysicalPosition<T> = Position<T, PhysicalCoord>;
/// The type of screen position.
pub type ScreenPosition<T> = Position<T, ScreenCoord>;

/// The type of logical size.
pub type LogicalSize<T> = Size<T, LogicalCoord>;
/// The type of physical size.
pub type PhysicalSize<T> = Size<T, PhysicalCoord>;

/// The type of a logical rectangle.
pub type LogicalRect<T> = Rect<T, LogicalCoord>;
/// The type of a physical rectangle.
pub type PhysicalRect<T> = Rect<T, PhysicalCoord>;

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

impl From<PhysicalRect<i32>> for RECT {
    #[inline]
    fn from(value: PhysicalRect<i32>) -> Self {
        RECT {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

/// This value is the base of logical position and size.
pub const DEFAULT_DPI: u32 = 96;

/// For converting to a logical coord value.
pub trait ToLogical<T> {
    type Output<U>;

    fn to_logical(&self, dpi: T) -> Self::Output<T>;
}

/// For converting to a physical coord value.
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

impl<T> ToLogical<T> for LogicalRect<T>
where
    T: Copy,
{
    type Output<U> = LogicalRect<U>;

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

impl<T> ToLogical<T> for PhysicalRect<T>
where
    T: num::Num + num::NumCast + Copy,
{
    type Output<U> = LogicalRect<T>;

    #[inline]
    fn to_logical(&self, dpi: T) -> Self::Output<T> {
        Rect::new(
            to_logical_value(self.left, dpi),
            to_logical_value(self.top, dpi),
            to_logical_value(self.right, dpi),
            to_logical_value(self.bottom, dpi),
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

impl<T> ToPhysical<T> for LogicalRect<T>
where
    T: num::Num + num::NumCast + Copy,
{
    type Output<U> = PhysicalRect<U>;

    #[inline]
    fn to_physical(&self, dpi: T) -> Self::Output<T> {
        Rect::new(
            to_physical_value(self.left, dpi),
            to_physical_value(self.top, dpi),
            to_physical_value(self.right, dpi),
            to_physical_value(self.bottom, dpi),
        )
    }
}

impl<T> ToPhysical<T> for PhysicalPosition<T>
where
    T: Copy,
{
    type Output<U> = PhysicalPosition<U>;

    #[inline]
    fn to_physical(&self, _dpi: T) -> Self::Output<T> {
        *self
    }
}

impl<T> ToPhysical<T> for PhysicalSize<T>
where
    T: Copy,
{
    type Output<U> = PhysicalSize<U>;

    #[inline]
    fn to_physical(&self, _dpi: T) -> Self::Output<T> {
        *self
    }
}

impl<T> ToPhysical<T> for PhysicalRect<T>
where
    T: Copy,
{
    type Output<U> = PhysicalRect<U>;

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

    #[test]
    fn logical_to_logical_rect() {
        let src = LogicalRect::new(6, 128, 256, 64);
        let dest = src.to_logical(DEFAULT_DPI * 2);
        assert!(src == dest);
    }

    #[test]
    fn logical_to_physical_rect() {
        let src = LogicalRect::new(6, 128, 256, 64);
        let dest = src.to_physical(DEFAULT_DPI * 2);
        assert!(src.left * 2 == dest.left);
        assert!(src.top * 2 == dest.top);
        assert!(src.right * 2 == dest.right);
        assert!(src.bottom * 2 == dest.bottom);
    }

    #[test]
    fn physical_to_logical_rect() {
        let src = PhysicalRect::new(6, 128, 256, 64);
        let dest = src.to_logical(DEFAULT_DPI * 2);
        assert!(src.left == dest.left * 2);
        assert!(src.top == dest.top * 2);
        assert!(src.right == dest.right * 2);
        assert!(src.bottom == dest.bottom * 2);
    }

    #[test]
    fn physical_to_physical_rect() {
        let src = PhysicalRect::new(6, 128, 256, 64);
        let dest = src.to_physical(DEFAULT_DPI * 2);
        assert!(src == dest);
    }
}
