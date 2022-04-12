//! Utilities used in the transition to uom-aware types
//!
//! Everything in this module should probably be removed after the transition is
//! complete.

use crate::{Length, Point, Vector, RatioPoint, RatioVec};


impl From<ncollide3d::math::Point<f32>> for Point {
    fn from(p: ncollide3d::math::Point<f32>) -> Self {
        use crate::uom::mm;
        let x: Length = mm(p.x);
        let y: Length = mm(p.y);
        let z: Length = mm(p.z);
        Self::new(x, y, z)
    }
}

impl From<ncollide3d::math::Vector<f32>> for Vector {
    fn from(v: ncollide3d::math::Vector<f32>) -> Self {
        use crate::uom::mm;
        let x: Length = mm(v.x);
        let y: Length = mm(v.y);
        let z: Length = mm(v.z);
        Self::new(x, y, z)
    }
}

impl From<Point> for ncollide3d::math::Point<f32> {
    fn from(p: Point) -> Self {
        use crate::uom::mm_;
        let x = mm_(p.x);
        let y = mm_(p.y);
        let z = mm_(p.z);
        Self::new(x, y, z)
    }
}

impl From<Vector> for ncollide3d::math::Vector<f32> {
    fn from(v: Vector) -> Self {
        use crate::uom::mm_;
        let x = mm_(v.x);
        let y = mm_(v.y);
        let z = mm_(v.z);
        Self::new(x, y, z)
    }
}

impl From<RatioVec> for ncollide3d::math::Vector<f32> {
    fn from(v: RatioVec) -> Self {
        use crate::uom::ratio_;
        let x = ratio_(v.x);
        let y = ratio_(v.y);
        let z = ratio_(v.z);
        Self::new(x, y, z)
    }
}

impl From<RatioPoint> for ncollide3d::math::Point<f32> {
    fn from(v: RatioPoint) -> Self {
        use crate::uom::ratio_;
        let x = ratio_(v.x);
        let y = ratio_(v.y);
        let z = ratio_(v.z);
        Self::new(x, y, z)
    }
}
