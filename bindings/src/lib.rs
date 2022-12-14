use pyo3::prelude::*;

type L = f32;
use petalo::{image::Image, fov::FOV, fom};
use units::{mm, todo::Intensityf32};

#[pyfunction]
#[pyo3(text_signature = "(n, /)")]
/// The naive, recursive fibonacci implementation
fn fib(n: usize) -> usize {
    if n < 2 { 1 }
    else     { fib(n-1) + fib(n-2) }
}

#[pymodule]
/// Module docstring works too!
fn fulano(_py_gil: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fib, m)?)?;
    m.add_function(wrap_pyfunction!(rust_enum_parameter, m)?)?;
    m.add_function(wrap_pyfunction!(roi, m)?)?;
    m.add_function(wrap_pyfunction!(fom_config, m)?)?;

    #[pyfn(m)]
    #[pyo3(name = "fab")]
    #[pyo3(text_signature = "(n, /)")]
    /// The iterative fibonacci implementation
    fn burp(_py_gil: Python, mut n: usize) -> usize {
        let (mut p, mut c) = (0,1);
        while n > 0 {
            let old_c = c;
            c += p;
            p = old_c;
            n -= 1;
        }
        c
    }

    m.add_class::<Lift>()?;
    m.add_class::<FomConfig>()?;

    Ok(())
}

#[pyclass]
struct FomConfig {
    cfg: fom::FomConfig,
    fov: FOV,
}

#[pymethods]
impl FomConfig {

    #[new]
    fn new(rois: Vec<(ROI, Intensityf32)>, bg_rois: Vec<ROI>, bg: Intensityf32, voxels: (usize, usize, usize), size: (L,L,L)) -> Self {
        let rois: Vec<(petalo::fom::ROI, Intensityf32)> = rois.into_iter()
            .map(|(r,i)| (pyroi_to_fomroi(r), i))
            .collect();
        let background_rois = bg_rois.into_iter().map(pyroi_to_fomroi).collect();

        let cfg = fom::FomConfig{ rois, background_rois, background_activity: bg};
        use units::mm;
        let size = (mm(size.0), mm(size.1), mm(size.2));
        FomConfig{ cfg, fov: FOV::new(size, voxels)}
    }

    /// Calculate CRC for a 60x60x60 voxel image
    fn crcs(&self, data: Vec<Intensityf32>) -> Vec<Intensityf32> {
        let image = Image::new(self.fov, data);
        image.foms(&self.cfg, true).crcs
    }

}


#[pyclass]
#[pyo3(text_signature = "(initial_height)")]
/// It's a Lift: it goes up and down
struct Lift {
    #[pyo3(get)]
    height: i32
}

#[pymethods]
impl Lift {

    #[new] // Signature goes on the struct
    fn new(initial_height: i32) -> Self { Self { height: initial_height }}

    fn up  (&mut self, n: usize) { self.height += n as i32 }
    fn down(&mut self, n: usize) { self.height -= n as i32 }

}

#[pyfunction]
fn roi(roi: ROI) -> String {
    use ROI::*;
    match roi {
        Sphere{x, y, z, r} => format!("S {} {} {} {}", x, y, z, r),
        CylinderX{y, z, r} => format!("X {} {} {}", y, z, r),
        CylinderY{x, z, r} => format!("Y {} {} {}", x, z, r),
        CylinderZ{x, y, r} => format!("Z {} {} {}", x, y, r),
    }
}


fn pyroi_to_fomroi(pyroi: ROI) -> petalo::fom::ROI {
    use              ROI as lr;
    use petalo::fom::ROI as fr;
    match pyroi {
        lr::Sphere {x,y,z,r} => fr::Sphere((mm(x), mm(y), mm(z)), mm(r)),
        lr::CylinderX{y,z,r} => fr::CylinderX(    (mm(y), mm(z)), mm(r)),
        lr::CylinderY{x,z,r} => fr::CylinderX(    (mm(x), mm(z)), mm(r)),
        lr::CylinderZ{x,y,r} => fr::CylinderZ(    (mm(x), mm(y)), mm(r)),
    }
}

#[pyfunction]
fn fom_config(rois: Vec<(ROI, Intensityf32)>, bg_rois: Vec<ROI>, bg: Intensityf32) -> String /*FomConfig*/ {
    let rois: Vec<(petalo::fom::ROI, Intensityf32)> = rois.into_iter()
        .map(|(r,i)| (pyroi_to_fomroi(r), i))
        .collect();
    let background_rois = bg_rois.into_iter().map(pyroi_to_fomroi).collect();

    let config = fom::FomConfig{ rois, background_rois, background_activity: bg};
    format!("{:?}", config)
}

#[derive(FromPyObject)]
enum ROI {
    Sphere{ x: L, y: L, z: L, r: L },
    CylinderZ{ x: L, y: L, r: L },
    CylinderY{ x: L, z: L, r: L },
    CylinderX{ y: L, z: L, r: L },
}


#[pyfunction]
/// Testing Rust enum conversion
fn rust_enum_parameter(e: RustyEnum) -> String {
    use RustyEnum::*;
    match e {
        Int(n)                   => format!("Int({})", n),
        String(s)                => format!("String(\"{}\")", s),
        IntTuple(a,b)            => format!("IntTuple({}, {})", a, b),
        StringIntTuple(a,b)      => format!("StringTuple(\"{}\", {})", a, b),
        Coordinates3d {x, y, z}  => format!("Coordinates3d({}, {}, {})", x,y,z),
        Coordinates2d {a:x, b:y} => format!("Coordinates2d({}, {})"    , x,y),
        //CatchAll(pyany)          => format!("CatchAll: {:?}", pyany),
    }
}

#[derive(FromPyObject)]
enum RustyEnum {
    Int(usize), // input is a positive int
    String(String), // input is a string
    IntTuple(usize, usize), // input is a 2-tuple with positive ints
    StringIntTuple(String, usize), // input is a 2-tuple with String and int
    Coordinates3d { // needs to be in front of 2d
        x: usize,
        y: usize,
        z: usize,
    },
    Coordinates2d { // only gets checked if the input did not have `z`
        #[pyo3(attribute("x"))]
        a: usize,
        #[pyo3(attribute("y"))]
        b: usize,
    },
    //#[pyo3(transparent)]
    //CatchAll(&'a PyAny), // This extraction never fails
}
