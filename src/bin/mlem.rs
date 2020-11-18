// ----------------------------------- CLI -----------------------------------
use structopt::StructOpt;

use petalo::utils::{parse_triplet, parse_range};

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "mlem", about = "TODO: describe what this does")]
pub struct Cli {

    /// Number of MLEM iterations to perform
    #[structopt(short, long, default_value = "5")]
    pub iterations: usize,

    /// Voxel box full-widths in mm
    #[structopt(short, long, parse(try_from_str = parse_triplet::<F>), default_value = "180,180,180")]
    pub size: (F, F, F),

    /// Number of voxels in each dimension
    #[structopt(short, long, parse(try_from_str = parse_triplet::<usize>), default_value = "60,60,60")]
     pub n_voxels: (usize, usize, usize),

    /// TOF resolution (sigma) in ps. If not supplied, TOF is ignored
    #[structopt(short = "r", long)]
     pub tof: Option<pet::Time>,

    /// Override automatic generation of image output file name
    #[structopt(short, long)]
     pub out_files: Option<String>,

    /// LORs to read in
    #[structopt(short = "f", long, default_value = "data/in/full_body_phantom_reco_combined.h5")]
    pub input_file: String,

    /// The dataset location inside the input file
    #[structopt(short, long, default_value = "reco_info/table")]
    pub dataset: String,

    /// Which rows of the input file should be loaded
    #[structopt(short, long, parse(try_from_str = parse_range::<usize>), default_value = "0..1000000")]
    pub event_range: std::ops::Range<usize>,

    /// Use the C version of the MLEM algorithm
    #[structopt(short = "c", long)]
    pub use_c: bool,

    /// Use true rather than reco LOR data
    #[structopt(long)]
    use_true: bool

}

// --------------------------------------------------------------------------------

use std::error::Error;
use std::path::PathBuf;
use std::fs::create_dir_all;

use petalo::weights as pet;
use petalo::io;

type F = pet::Length;


fn main() -> Result<(), Box<dyn Error>> {

    let args = Cli::from_args();

    println!("Float precision: {} bits", petalo::weights::PRECISION);

    // Set up progress reporting and timing
    use std::time::Instant;
    let mut now = Instant::now();

    let mut report_time = |message| {
        println!("{}: {} ms", message, now.elapsed().as_millis());
        now = Instant::now();
    };

    // Read event data from disk into memory
    let                      Cli{ input_file, dataset, event_range, use_true, .. } = args.clone();
    let io_args = io::hdf5::Args{ input_file, dataset, event_range, use_true     };
    let measured_lors = io::hdf5::read_lors(io_args)?;
    report_time("Loaded LOR data from local disk");

    // Define extent and granularity of voxels
    let vbox = pet::VoxelBox::new(args.size, args.n_voxels);
    // TODO: sensitivity matrix, all ones for now
    let sensitivity_matrix = pet::Image::ones(vbox).data;
    // TODO: noise
    let noise = pet::Noise;

    let file_pattern = guess_filename(&args);

    // If the directory where results will be written does not exist yet, make it
    create_dir_all(PathBuf::from(format!("{}_00.raw", file_pattern)).parent().unwrap())?;

    // Perform MLEM iterations
    if args.use_c {
        run_cmlem(&args, &measured_lors)
    } else {
        for (n, image) in (pet::Image::mlem(vbox, &measured_lors, args.tof, &sensitivity_matrix, &noise))
            .take(args.iterations)
            .enumerate() {
                report_time("iteration");
                let data: ndarray::Array3<F> = image.data;
                let path = PathBuf::from(format!("{}_{:02}.raw", file_pattern, n));
                write(data, &path)?;
                report_time("Wrote raw bin");
                // TODO: step_by for print every
            }
    }
    Ok(())
}

fn write(data: ndarray::Array3<F>, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    use petalo::io::raw::write;
    #[cfg(not(feature = "f64"))] write(data.t().iter().copied()          , path)?;
    #[cfg    (feature = "f64") ] write(data.t().iter().map(|&x| x as f32), path)?;
    Ok(())
}


fn guess_filename(args: &Cli) -> String {
    if let Some(pattern) = &args.out_files {
        pattern.to_string()
    } else {
        let c = if args.use_c { "c" } else { "" };
        let (nx, ny, nz) = args.n_voxels;
        let tof = args.tof.map_or(String::from("OFF"), |x| format!("{:.0}", x));
        format!("data/out/{c}mlem/{nx}_{ny}_{nz}_tof_{tof}",
                c=c, nx=nx, ny=ny, nz=nz, tof=tof)
    }
}

// ---- Use the original tofpet3d libmlem (C version), instead of our own Rust version ---

// TODO: this conversion function should really live in the cmlem package, but
// that would require cmlem to depend on the petalo package, because that's
// where types like LOR are defined ... but this crate is currently in the
// petalo package, and it needs to depend on cmlem to call the cmlem function,
// which introduces a circular package dependency, which cargo does not allow.
// The solution is to move this mlem binary crate out of the petalo package, but
// let's just get it working at all, for the time being, and reorganize the
// packages later

use petalo::weights::{LOR};

fn run_cmlem(
    args: &Cli,
    lors: &Vec<LOR>
) {
    // Image dimensions
    let (nx, ny, nz) = args.n_voxels;
    let (sx, sy, sz) = args.size;

    // decompose LORs into separate vectors
    let mut x1 = vec![]; let mut y1 = vec![]; let mut z1 = vec![]; let mut t1 = vec![];
    let mut x2 = vec![]; let mut y2 = vec![]; let mut z2 = vec![]; let mut t2 = vec![];
    for lor in lors {
        x1.push(lor.p1.x);
        y1.push(lor.p1.y);
        z1.push(lor.p1.z);
        t1.push(lor.t1);
        x2.push(lor.p2.x);
        y2.push(lor.p2.y);
        z2.push(lor.p2.z);
        t2.push(lor.t2);
    }

    // Add underscore to separate base name from suffix (to match what happens
    // in the Rust version)
    let mut files = guess_filename(&args);
    files.push('_');
    files.push('0'); // Leading zero too!

    // TODO: Dummy sensitivity matrix, for now
    let sensitivity_matrix = vec![1.0; nx * ny * nz];

    #[cfg(not(feature = "f64"))]
    cmlem::cmlem(
        args.iterations,
        args.tof.is_some(),
        args.tof.unwrap_or(0.0),
        if sx != sy { panic!("cmlem requires x and y FOVs to be equal") } else { sx },
        sz,
        if nx != ny { panic!("cmlem requires Nx and Ny to be equal") } else { nx },
        nz,
        lors.len(),
        x1, y1, z1, t1,
        x2, y2, z2, t2,
        sensitivity_matrix,
        files,
        1, // save every iteration
    );
}
