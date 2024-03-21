// Copyright 2024 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! f32 versions of cubics, tweaked for subdivision

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }

    pub fn from_kurbo(p: kurbo::Point) -> Self {
        Point::new(p.x as f32, p.y as f32)
    }

    pub fn hypot(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn hypot2(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn atan2(self) -> f32 {
        self.y.atan2(self.x)
    }
}

// Note: we don't have separate Vec2 like kurbo
impl core::ops::Add for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Point {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl core::ops::Sub for Point {
    type Output = Point;

    fn sub(self, rhs: Point) -> Point {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl core::ops::Mul<f32> for Point {
    type Output = Point;

    fn mul(self, rhs: f32) -> Point {
        Point::new(self.x * rhs, self.y * rhs)
    }
}

pub struct Cubic {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl Cubic {
    pub fn new(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Cubic { p0, p1, p2, p3 }
    }

    /// Evaluate point and derivative.
    ///
    /// Note that the second value returned is 1/3 the actual derivative,
    /// to reduce multiplication.
    pub fn eval_and_deriv(&self, t: f32) -> (Point, Point) {
        let m = 1.0 - t;
        let mm = m * m;
        let mt = m * t;
        let tt = t * t;
        let p =
            self.p0 * (mm * m) + (self.p1 * (3.0 * mm) + self.p2 * (3.0 * mt) + self.p3 * tt) * t;
        let q =
            (self.p1 - self.p0) * mm + (self.p2 - self.p1) * (2.0 * mt) + (self.p3 - self.p2) * tt;
        (p, q)
    }
}
