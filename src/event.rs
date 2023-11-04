use crate::*;

#[derive(Debug)]
pub struct Draw {
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    Draw(Draw),
    Closed,
}

