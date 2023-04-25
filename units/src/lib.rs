/// The `uom`-based system of units used by petalo
///
/// It uses `mm` and `ps` as the base units of `Length` and `Time` (as opposed
/// to the standard and default `m` and `s`), on order to match the meaning of
/// the floats written out by the Geant4-based Monte Carlo data.
///
/// TODO: write more documentation


// Make the version of `uom` that is used here, accessible to other crates in
// the workspace. The problem is that the versions of `uom` declared as
// dependencies in differenc crates in the workspace, can diverge, and then we
// get annoying compilation errors. So, for now, we agree to use *this* version
// everywhere in the production code. The other version is used only in the
// `uom` example, which I want to leave in it's current prominent place, rather
// than moving it into this crate.
// TODO: this shouldn't be necessary any more once
// https://github.com/rust-lang/cargo/issues/8415 is stabilized.
pub use uom;

pub mod todo;

use uom::si::Dimension;
pub type InvertDimension<D> = uom::si::ISQ<
    <<D as Dimension>::L  as uom::lib::ops::Neg>::Output,
    <<D as Dimension>::M  as uom::lib::ops::Neg>::Output,
    <<D as Dimension>::T  as uom::lib::ops::Neg>::Output,
    <<D as Dimension>::I  as uom::lib::ops::Neg>::Output,
    <<D as Dimension>::Th as uom::lib::ops::Neg>::Output,
    <<D as Dimension>::N  as uom::lib::ops::Neg>::Output,
    <<D as Dimension>::J  as uom::lib::ops::Neg>::Output>;

/// A system of units with millimetres and picoseconds replacing the default
/// metres and seconds as the base units of `Length` and `Time`
/// Useful for bitwise compatibility with data generated by Geant4.
pub mod mmps {
  pub mod f32 {
    use uom::{ISQ, system};
    ISQ!(uom::si, f32, (millimeter, kilogram, picosecond, ampere, kelvin, mole, candela));

    /// The full circle constant (τ) Equal to 2π.
    pub const TWOPI: Angle = Angle {
        dimension: std::marker::PhantomData,
        units: std::marker::PhantomData,
        value: std::f32::consts::TAU,
    };
  }

  pub mod i32 {
    use uom::{ISQ, system};
    ISQ!(uom::si, i32, (millimeter, kilogram, picosecond, ampere, kelvin, mole, candela));
  }

  pub mod usize {
    use uom::{ISQ, system};
    ISQ!(uom::si, usize, (millimeter, kilogram, picosecond, ampere, kelvin, mole, candela));
  }

}

use uom::typenum::{P4, P2, N1, Z0};
pub type PerLength   = Quantity<InvertDimension<uom::si::length::Dimension>, mmps::f32::Units, f32>;
pub type AreaPerMass = Quantity<uom::si::ISQ<P2, N1, Z0, Z0, Z0, Z0, Z0>   , mmps::f32::Units, f32>;
pub type Length4     = Quantity<uom::si::ISQ<P4, Z0, Z0, Z0, Z0, Z0, Z0>   , mmps::f32::Units, f32>;

//use uom::fmt::DisplayStyle::Abbreviation;
pub use uom::si::Quantity;
pub use mmps::f32::{Angle, Area, TWOPI, Length, Time, Velocity, Ratio, Mass, Energy};
mod units {
  pub use uom::si::{length  ::{nanometer, millimeter, centimeter},
                    time    ::{nanosecond, picosecond},
                    mass    ::kilogram,
                    velocity::meter_per_second,
                    ratio   ::ratio,
                    angle   ::{radian, revolution},
                    energy  ::kiloelectronvolt,
  };
}

// Making uom quantities from float literals is very long-winded, so provide
// some pithily-named convenience constructors. These would probably have to be
// packed up in a constructor module in real life.

/// Generate a pair of functions for converting between f32 and uom quantities.
///
/// wrap!(WRAP_NAME UNWRAP_NAME QUANTITY UNIT);
///
/// The wrapping function is called WRAP_NAME and returns QUANTITY by
/// interpreting its argument as UNIT. The function UNWRAP_NAME is the inverse
/// of WRAP_NAME.
macro_rules! wrap {
  ($wrap_name:ident $unwrap_name:ident $quantity:ident $unit:ident ) => {
    #[allow(nonstandard_style)]
    pub fn   $wrap_name(x: f32) -> $quantity { $quantity::new::<units::$unit>(x) }
    #[allow(nonstandard_style)]
    pub fn $unwrap_name(x: $quantity) -> f32 {          x.get::<units::$unit>( ) }
  };
}

wrap!(cm     cm_     Length         centimeter);
wrap!(mm     mm_     Length         millimeter);
wrap!(nm     nm_     Length          nanometer);
wrap!(ns     ns_     Time           nanosecond);
wrap!(ps     ps_     Time           picosecond);
wrap!(m_s    m_s_    Velocity meter_per_second);
wrap!(kg     kg_     Mass             kilogram);
wrap!(ratio  ratio_  Ratio               ratio);
wrap!(radian radian_ Angle              radian);
wrap!(turn   turn_   Angle          revolution);
wrap!(keV    keV_    Energy   kiloelectronvolt);

pub fn mm_ps (x: f32) -> Velocity { m_s (x / m_s(1.0).value) }
pub fn mm_ps_(x: Velocity) -> f32 { m_s_(x * m_s(1.0).value) }


#[macro_export]
macro_rules! in_base_unit {
  ($value:expr) => {
    $crate::Quantity {
      dimension: std::marker::PhantomData,
      units: std::marker::PhantomData,
      value: $value,
    }
  };
}


#[doc(hidden)]
pub use float_eq;

#[macro_export]
macro_rules! assert_uom_eq {
    ($unit:ident, $lhs:expr, $rhs:expr, $algo:ident <= $tol:expr) => (
        $crate::float_eq::assert_float_eq!($lhs.get::<$unit>(), $rhs.get::<$unit>(), $algo <= $tol)
    );
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let v = vec![mm(1.0), cm(1.0)];
        let total: Length = v.into_iter().sum();
        use units::nanometer;
        assert_uom_eq!(nanometer, total, mm(11.0), ulps <= 1);
    }
}



#[allow(clippy::excessive_precision)]
pub const C: Velocity = in_base_unit!(0.299_792_458);
// `f32` will truncate this to        0.299_792_47

#[cfg(test)]
mod test_speed_of_light {
    use super::*;
    use float_eq::assert_float_eq;

    #[test]
    #[allow(clippy::excessive_precision)]
    fn test_speed_of_light() {
        println!("C visual check: {:?}", C);
        assert_float_eq!(  m_s_(C),   299_792_458.0, ulps <= 1);
        assert_float_eq!(mm_ps_(C), 0.299_792_458  , ulps <= 1);
    }
}
