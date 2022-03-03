use ndhistogram::{ndhistogram, axis::{Uniform, Cyclic}, Histogram, HistND};
use crate::weights::LOR;
use crate::types::Point;

// TODO: replace with uom
type Length = f32;
type Ratio  = f32;
type Angle  = f32;

/// Distinguish between true, scatter and random prompt signals
pub enum Prompt { True, Scatter, Random }

pub trait Lorogram {
    fn fill              (&mut self, lor: &LOR);
    fn value             (&    self, lor: &LOR) -> usize;
    fn interpolated_value(&    self, lor: &LOR) -> Ratio;
}

pub struct Scattergram {
    trues  : Box<dyn Lorogram>,
    scatters:Box<dyn Lorogram>,
}

impl Scattergram {

    pub fn new(make_empty_lorogram: &(dyn Fn() -> Box<dyn Lorogram>)) -> Self {
        let trues    = make_empty_lorogram();
        let scatters = make_empty_lorogram();
        Self { trues, scatters }
    }

    pub fn fill(&mut self, kind: Prompt, lor: &LOR) {
        match kind {
            Prompt::True    => self.trues.   fill(lor),
            Prompt::Scatter => self.scatters.fill(lor),
            Prompt::Random  => panic!("Not expecting any random events yet."),
        }
    } 

    /// Multiplicative contribution of scatters to trues, in nearby LORs.
    ///
    /// `(scatters + trues) / trues`
    pub fn value(&self, lor: &LOR) -> Ratio {
        let trues = self.trues.value(lor);
        if trues > 0 {
            let scatters: f32 = self.scatters.value(lor) as f32;
            let trues = trues as f32;
            (scatters + trues) / trues
        } else { 1.0 }
    }

    pub fn triplet(&self, lor: &LOR) -> (Ratio, f32, f32) {
        let trues = self.trues.value(lor);
        if trues > 0 {
            let scatters: f32 = self.scatters.value(lor) as f32;
            let trues = trues as f32;
            ((scatters + trues) / trues, trues, scatters)
        } else { (1.0, 0.0, self.scatters.value(lor) as f32) }
    }
}
// --------------------------------------------------------------------------------
type Uniform1DHist = HistND<(Uniform<Length>,), usize>;

pub struct JustZ {
    histogram: Uniform1DHist,
}

impl JustZ {
    pub fn new(l: Length, nbins: usize) -> Self {
        Self { histogram: ndhistogram!(Uniform::new(nbins, -l/2.0, l/2.0); usize) }
    }
}

impl Lorogram for JustZ {
    fn fill (&mut self, lor: &LOR)          {  self.histogram.fill (&z_of_midpoint(lor)); }
    fn value(&    self, lor: &LOR) -> usize { *self.histogram.value(&z_of_midpoint(lor)).unwrap_or(&0) }
    fn interpolated_value(&self, lor: &LOR) -> f32   { todo!() }
}

fn z_of_midpoint(LOR {p1, p2, ..}: &LOR) -> Length { (p1.z + p2.z) / 2.0 }

impl From<((f32, f32, f32), (f32, f32, f32))> for LOR {
    fn from(((x1,y1,z1), (x2,y2,z2)): ((f32, f32, f32), (f32, f32, f32))) -> Self {
        Self { p1: Point::new(x1,y1,z1), p2: Point::new(x2,y2,z2), dt: 0.0  }
    }
}

#[cfg(test)]
mod test_just_z {
    use super::*;

    #[test]
    fn retrieve() {
        let mut lg = JustZ::new(1000.0, 10);
        lg.fill         (&LOR::from(((0.0, 0.0, 111.0), (0.0, 0.0, 555.0))));
        let n = lg.value(&LOR::from(((1.0, 2.0, 222.0), (9.0, 8.0, 444.0))));
        assert_eq!(n, 1);
    }
}
// --------------------------------------------------------------------------------
pub struct JustR {
    histogram: Uniform1DHist,
}

impl JustR {
    pub fn new(r: Length, nbins: usize) -> Self {
        Self { histogram: ndhistogram!(Uniform::new(nbins, 0.0, r); usize) }
    }
}

impl Lorogram for JustR {
    fn fill (&mut self, lor: &LOR)          {  self.histogram.fill (&distance_from_z_axis(lor));}
    fn value(&    self, lor: &LOR) -> usize { *self.histogram.value(&distance_from_z_axis(lor)).unwrap_or(&0) }

    fn interpolated_value(&    self, lor: &LOR) -> Ratio {
        todo!()
    }
}

fn distance_from_z_axis(LOR{ p1, p2, .. }: &LOR) -> Length {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let x1 = p1.x;
    let y1 = p1.y;
    (dx * y1 - dy * x1).abs() / (dx*dx + dy*dy).sqrt()
}
// --------------------------------------------------------------------------------
type Cyclic1DHist = HistND<(Cyclic<f32>,), usize>;

pub struct JustPhi {
    histogram: Cyclic1DHist,
}

impl JustPhi {
    pub fn new(nbins: usize) -> Self {
        Self { histogram: ndhistogram!(Cyclic::new(nbins, 0.0, std::f32::consts::PI); usize) }
    }
}

impl Lorogram for JustPhi {
    fn fill (&mut self, lor: &LOR)          {  self.histogram.fill (&phi(lor)); }
    fn value(&    self, lor: &LOR) -> usize { *self.histogram.value(&phi(lor)).unwrap_or(&0) }

    fn interpolated_value(&    self, lor: &LOR) -> Ratio {
        todo!()
    }
}

fn phi(LOR{ p1, p2, .. }: &LOR) -> Angle {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    phi_of_x_y(dx, dy)
}

fn phi_of_x_y(x: Length, y: Length) -> Angle { y.atan2(x) }
// --------------------------------------------------------------------------------
pub struct JustDeltaZ {
    histogram: Uniform1DHist,
}

impl JustDeltaZ {
    pub fn new(dz_max: Length, nbins: usize) -> Self {
        Self { histogram: ndhistogram!(Uniform::new(nbins, 0.0, dz_max); usize) }
    }
}

impl Lorogram for JustDeltaZ {
    fn fill (&mut self, lor: &LOR)          {  self.histogram.fill (&delta_z(lor)); }
    fn value(&    self, lor: &LOR) -> usize { *self.histogram.value(&delta_z(lor)).unwrap_or(&0) }

    fn interpolated_value(&    self, lor: &LOR) -> Ratio {
        todo!()
    }
}

fn delta_z(LOR{p1, p2, ..}: &LOR) -> Length { (p1.z - p2.z).abs() }
// --------------------------------------------------------------------------------
type Uniform2DHist = HistND<(Uniform<Length>, Uniform<Length>), usize>;

pub struct ZAndDeltaZ {
    histogram: Uniform2DHist
}

impl ZAndDeltaZ {
    pub fn new(l: Length, nbins_z: usize, dz_max: Length, nbins_dz: usize) -> Self {
        Self {
            histogram: ndhistogram!(
                Uniform::new(nbins_z, 0.0, dz_max),
                Uniform::new(nbins_dz, -l/2.0, l/2.0);
                usize)
        }
    }
}

impl Lorogram for ZAndDeltaZ {
    fn fill (&mut self, lor: &LOR)          {  self.histogram.fill (&(z_of_midpoint(lor), delta_z(lor))); }
    fn value(&    self, lor: &LOR) -> usize { *self.histogram.value(&(z_of_midpoint(lor), delta_z(lor))).unwrap_or(&0) }

    fn interpolated_value(&    self, lor: &LOR) -> Ratio {
        todo!()
    }
}
