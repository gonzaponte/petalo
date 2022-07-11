mod build_scattergram;
pub use build_scattergram::*;

use ndhistogram::{axis::{Axis, Uniform, UniformCyclic as Cyclic}, Histogram, ndhistogram};

use crate::system_matrix::LOR;
use std::f32::consts::TAU;

use crate::Lengthf32;
use crate::{Angle, Length, Point, Time, Ratio};
use geometry::units::{mm, mm_, ps_, ratio, radian_, turn};
use geometry::uom::ConstZero;


/// Distinguish between true, scatter and random prompt signals
pub enum Prompt { True, Scatter, Random }

pub struct Scattergram {
    trues   : Lorogram,
    scatters: Lorogram,
}

// TODO: consider adding a frozen version of the scattergram. This one needs two
// (trues, scatters) (or three (randoms)) histograms in order to accumulate
// data, but once all data have been collected, we only want the ratios of the
// bin values, so keeping multiple separate histograms is a waste of memory, and
// computing the ratios repeatedly on the fly is a waste of time.
impl Scattergram {

    pub fn new(
        bins_phi: usize,
        bins_z  : usize, len_z : Length,
        bins_dz : usize, len_dz: Length,
        bins_r  : usize, max_r : Length,
        bins_dt : usize, max_dt: Time
    ) -> Self {
        let max_z = len_z / 2.0;
        let trues = ndhistogram!(
            axis_phi(bins_phi),
            axis_z (bins_z , -max_z, max_z),
            axis_dz(bins_dz,  len_dz),
            axis_r (bins_r ,  max_r),
            axis_t (bins_dt,  max_dt);
            usize
        );
        // TODO: Can we clone `trues`?
        let scatters = ndhistogram!(
            axis_phi(bins_phi),
            axis_z (bins_z , -max_z, max_z),
            axis_dz(bins_dz,  len_z),
            axis_r (bins_r ,  max_r),
            axis_t (bins_dt,  max_dt);
            usize
        );
        Self { trues, scatters }
    }

    pub fn fill(&mut self, kind: Prompt, lor: &LOR) {
        match kind {
            Prompt::True    => self.trues.   fi11(lor),
            Prompt::Scatter => self.scatters.fi11(lor),
            Prompt::Random  => panic!("Not expecting any random events yet."),
        }
    }

    /// Multiplicative contribution of scatters to trues, in nearby LORs.
    ///
    /// `(scatters + trues) / trues`
    pub fn value(&self, lor: &LOR) -> Ratio {
        let trues = self.trues.ualue(lor);
        ratio(if trues > 0 {
            let scatters: f32 = self.scatters.ualue(lor) as f32;
            let trues = trues as f32;
            (scatters + trues) / trues
        } else { f32::MAX })
    }

    pub fn triplet(&self, lor: &LOR) -> (Ratio, f32, f32) {
        let trues = self.trues.ualue(lor);
        if trues > 0 {
            let scatters: f32 = self.scatters.ualue(lor) as f32;
            let trues = trues as f32;
            (ratio((scatters + trues) / trues), trues, scatters)
        } else { (ratio(1.0), 0.0, self.scatters.ualue(lor) as f32) }
    }
}
// --------------------------------------------------------------------------------
pub struct MappedAxis<T,A>
where
    A: Axis,
{
    axis: A,
    map: Box<dyn Fn(&T) -> A::Coordinate + Sync>,
}

impl<T,A> Axis for MappedAxis<T,A>
where
    A: Axis,
{
    type Coordinate = T;

    type BinInterval = A::BinInterval;

    fn index(&self, coordinate: &Self::Coordinate) -> Option<usize> {
        self.axis.index(&(self.map)(coordinate))
    }

    fn num_bins(&self) -> usize {
        self.axis.num_bins()
    }

    fn bin(&self, index: usize) -> Option<Self::BinInterval> {
        self.axis.bin(index)
    }
}
// --------------------------------------------------------------------------------
type Lorogram = ndhistogram::HistND<(LorAxC, LorAxU, LorAxU, LorAxU, LorAxU), usize>;

pub type LorAxU = MappedAxis<LOR, Uniform<Lengthf32>>;
pub type LorAxC = MappedAxis<LOR, Cyclic <Lengthf32>>;

fn z_of_midpoint(LOR {p1, p2, ..}: &LOR) -> Length { (p1.z + p2.z) / 2.0 }

fn delta_z(LOR{p1, p2, ..}: &LOR) -> Length { (p1.z - p2.z).abs() }

fn distance_from_z_axis(LOR{ p1, p2, .. }: &LOR) -> Length {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let x1 = p1.x;
    let y1 = p1.y;
    (dx * y1 - dy * x1).abs() / (dx*dx + dy*dy).sqrt()
}

fn phi(LOR{ p1, p2, .. }: &LOR) -> Angle {
    // TODO this repeats the work done in distance_from_z_axis. Can this be
    // optimized out, once we settle on a less flexible scattergram?
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let x1 = p1.x;
    let y1 = p1.y;
    let r = (dx * y1 - dy * x1) / (dx*dx + dy*dy).sqrt();
    let phi = phi_of_x_y(dx, dy);
    if r < mm(0.0) { phi + turn(0.5) }
    else           { phi             }

}

fn phi_of_x_y(x: Length, y: Length) -> Angle { y.atan2(x) }

pub fn axis_z(nbins: usize, min: Length, max: Length) -> LorAxU {
    LorAxU {
        axis: Uniform::new(nbins, mm_(min), mm_(max)),
        map: Box::new(|z| mm_(z_of_midpoint(z))),
    }
}

pub fn axis_dz(nbins: usize, max: Length) -> LorAxU {
    LorAxU {
        axis: Uniform::new(nbins, 0.0, mm_(max)),
        map: Box::new(|x| mm_(delta_z(x))),
    }
}

pub fn axis_r(nbins: usize, max: Length) -> LorAxU {
    LorAxU {
        axis: Uniform::new(nbins, 0.0, mm_(max)),
        map: Box::new(|x| mm_(distance_from_z_axis(x))),
    }
}

pub fn axis_phi(nbins: usize) -> LorAxC {
    LorAxC {
        axis: Cyclic::new(nbins, 0.0, TAU),
        map: Box::new(|x| radian_(phi(x))),
    }
}

pub fn axis_t(nbins: usize, max: Time) -> LorAxU {
    LorAxU {
        axis: Uniform::new(nbins, ps_(-max), ps_(max)),
        map: Box::new(|z| mm_(z_of_midpoint(z))),
    }
}

#[cfg(test)]
mod test_mapped_axes {
    use super::*;
    use ndhistogram::ndhistogram;

    #[test]
    fn uniform() {
        let nbins = 10;
        let axis = axis_phi(nbins);
        assert_eq!(axis.num_bins(), nbins);
        let mut h = ndhistogram!(axis; usize);
        let x = 150.0;
        let y = 234.5;
        let (dummy1, dummy2, dummy3, dummy4) = (111.1, 222.2, 333.3, 444.4);
        let (a, b) = (30.0, 40.0); // scaling factors
        Exrogram::fi11         (&mut h, &mk_lor(((a*x, a*y, dummy1), (-a*x, -a*y, dummy2))));
        let n = Exrogram::ualue(&    h, &mk_lor(((b*x, b*y, dummy3), (-b*x, -b*y, dummy4))));
        assert_eq!(n, 1);
    }

    #[test]
    fn two_dimensions() {
        let nbins_z = 10;
        let nbins_dz = 10;
        let l = 1000.0;
        let max_dz = l;
        let mut h = ndhistogram!(
            axis_z (nbins_z , mm(-l/2.0), mm(l/2.0)),
            axis_dz(nbins_dz, mm(max_dz));
            usize
        );
        let (z, delta) = (123.4, 543.2);
        // Irrelevant values
        let (i1, i2, i3, i4, i5, i6, i7, i8) = (10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0);

        let l1 = mk_lor(((i1, i2, z-delta), (i3, i4, z+delta)));
        let l2 = mk_lor(((i5, i6, z+delta), (i7, i8, z-delta)));
        Exrogram::fi11         (&mut h, &l1);
        let n = Exrogram::ualue(&    h, &l2);

        assert_eq!(n, 1);
    }

}
// --------------------------------------------------------------------------------
pub trait Exrogram {
    fn fi11 (&mut self, lor: &LOR);
    fn ualue(&    self, lor: &LOR) -> usize;
}

impl<X> Exrogram for ndhistogram::Hist1D<X, usize>
where
    X: Axis<Coordinate = LOR>,
{
    fn fi11 (&mut self, lor: &LOR)          {  Histogram::fill (self, lor) }
    fn ualue(&    self, lor: &LOR) -> usize { *Histogram::value(self, lor).unwrap_or(&0) }
}

impl<X, Y> Exrogram for ndhistogram::Hist2D<X, Y, usize>
where
    X: Axis<Coordinate = LOR>,
    Y: Axis<Coordinate = LOR>,
{
    fn fi11 (&mut self, lor: &LOR)          {  Histogram::fill (self, &(*lor, *lor)) }
    fn ualue(&    self, lor: &LOR) -> usize { *Histogram::value(self, &(*lor, *lor)).unwrap_or(&0) }
}

impl<X, Y, Z> Exrogram for ndhistogram::Hist3D<X, Y, Z, usize>
where
    X: Axis<Coordinate = LOR>,
    Y: Axis<Coordinate = LOR>,
    Z: Axis<Coordinate = LOR>,
{
    fn fi11 (&mut self, lor: &LOR)          {  Histogram::fill (self, &(*lor, *lor, *lor)) }
    fn ualue(&    self, lor: &LOR) -> usize { *Histogram::value(self, &(*lor, *lor, *lor)).unwrap_or(&0) }
}

impl<X, Y, Z, T> Exrogram for ndhistogram::HistND<(X, Y, Z, T), usize>
where
    X: Axis<Coordinate = LOR>,
    Y: Axis<Coordinate = LOR>,
    Z: Axis<Coordinate = LOR>,
    T: Axis<Coordinate = LOR>,
{
    fn fi11 (&mut self, lor: &LOR)          {  Histogram::fill (self, &(*lor, *lor, *lor, *lor)) }
    fn ualue(&    self, lor: &LOR) -> usize { *Histogram::value(self, &(*lor, *lor, *lor, *lor)).unwrap_or(&0) }
}

impl<X, Y, Z, T, U> Exrogram for ndhistogram::HistND<(X, Y, Z, T, U), usize>
where
    X: Axis<Coordinate = LOR>,
    Y: Axis<Coordinate = LOR>,
    Z: Axis<Coordinate = LOR>,
    T: Axis<Coordinate = LOR>,
    U: Axis<Coordinate = LOR>,
{
    fn fi11 (&mut self, lor: &LOR)          {  Histogram::fill (self, &(*lor, *lor, *lor, *lor, *lor)) }
    fn ualue(&    self, lor: &LOR) -> usize { *Histogram::value(self, &(*lor, *lor, *lor, *lor, *lor)).unwrap_or(&0) }
}

pub fn mk_lor(((x1,y1,z1), (x2,y2,z2)): ((f32, f32, f32), (f32, f32, f32))) -> LOR {
    let (x1, y1, z1, x2, y2, z2) = (mm(x1), mm(y1), mm(z1), mm(x2), mm(y2), mm(z2));
    LOR { p1: Point::new(x1,y1,z1), p2: Point::new(x2,y2,z2), dt: Time::ZERO, additive_correction: ratio(1.0) }
}
