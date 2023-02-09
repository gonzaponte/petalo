use petalo::config::mlem::AttenuationCorrection as AC;
// ----------------------------------- CLI -----------------------------------
use clap::Parser;

use petalo::config;

#[derive(clap::Parser, Debug, Clone)]
#[clap(name = "mlem", about = "Maximum Likelyhood Expectation Maximization")]
pub struct Cli {

    /// MLEM config file
    pub config_file: PathBuf,

    /// Directory in which results should be written
    pub output_directory: PathBuf,

    /// Maximum number of rayon threads used by MLEM
    #[clap(short = 'j', long, default_value = "4")]
    pub mlem_threads: usize,

    // TODO: if we keep this, we need to come up with good names
    /// Rayon threads for filling scattergram [default: mlem-threads]
    #[clap(short = 'k', long)]
    pub scattergram_threads: Option<usize>,

}

// --------------------------------------------------------------------------------

use std::error::Error;
use std::path::PathBuf;
use std::fs::create_dir_all;

use units::{Length, mm_};
use petalo::{
    fov::FOV,
    image::Image,
    io,
    mlem::Osem,
    utils::timing::Progress
};


fn main() -> Result<(), Box<dyn Error>> {

    let args = Cli::parse();
    let config = config::mlem::read_config_file(args.config_file.clone());
    unsafe { petalo::mlem::N_MLEM_THREADS = args.mlem_threads; }

    // Set up progress reporting and timing
    let mut progress = Progress::new();

    // Check that output directory is writable. Do this *before* expensive
    // setup, so it fails early
    // If the directory where results will be written does not exist yet, make it
    create_dir_all(&args.output_directory)
        .unwrap_or_else(|_| panic!("Cannot write in output directory `{}`", args.output_directory.display()));
    // Copy config file to output directory, in order to preserve metadata
    std::fs::copy(
        &args.config_file,
        args.output_directory.join("mlem-config.toml")
    )?;
    // Show configuration being run
    println!("Configuration:\n{config}");

    // Define field of view extent and voxelization
    let fov = FOV::new(config.fov.size, config.fov.nvoxels);

    let scattergram = config.scatter_correction.as_ref().and_then(Into::into);
    progress.done_with_message("Startup");

    let sensitivity_image =
        if let Some(AC { sensitivity_image: path }) = config.attenuation_correction.as_ref() {
            let image = Image::from_raw_file(path)
                .unwrap_or_else(|_| panic!("Cannot read sensitivity image {:?}", path.display()));
            assert_image_sizes_match(&image, config.fov.nvoxels, config.fov.size);
            progress.done_with_message("Loaded sensitivity image");
            Some(image)
        } else { None };

    progress.startln("Loading LORs from file");
    let scattergram_threads = args.scattergram_threads.unwrap_or(args.mlem_threads);
    let measured_lors = io::hdf5::read_lors(&config, scattergram, scattergram_threads)?;
    progress.done_with_message("Loaded LORs from file");

    let pool = rayon::ThreadPoolBuilder::new().num_threads(args.mlem_threads).build()?;
    println!("MLEM: Using up to {} threads.", args.mlem_threads);
    pool.install(|| {
        for (image, Osem{iteration, subset, ..}) in (petalo::mlem::mlem(fov, &measured_lors, config.tof, sensitivity_image, config.iterations.subsets))
            .take(config.iterations.number * config.iterations.subsets) {
                progress.done_with_message(&format!("Iteration {iteration:2}-{subset:02}"));
                let path = PathBuf::from(format!("{}{iteration:02}-{subset:02}.raw", args.output_directory.display()));
                petalo::io::raw::Image3D::from(&image).write_to_file(&path).unwrap();
                progress.done_with_message("                               Wrote raw bin");
                // TODO: step_by for print every
            }
    });

    Ok(())
}


type FovSize = (Length, Length, Length);
type NVoxels = (usize , usize , usize );

/// Panic if the image size does not match the specified values
fn assert_image_sizes_match(image: &Image, nvoxels: NVoxels, fov_size: FovSize) {
    use float_eq::float_eq;
    let size = image.fov.half_width;
    let (idx, idy, idz) = (size[0]*2.0, size[1]*2.0, size[2]*2.0);
    let [inx, iny, inz] = image.fov.n;
    let (enx, eny, enz) = nvoxels;
    let (edx, edy, edz) = fov_size;
    // Unwrap uom, to make float_eq! work
    let ids = [mm_(idx), mm_(idy), mm_(idz)];
    let eds = [mm_(edx), mm_(edy), mm_(edz)];
    if ! ((enx, eny, enz) == (inx, iny, inz) && float_eq!(eds, ids, ulps_all <= 1)) {
        // TODO enable use of density images with different
        // pixelizations as long as they cover the whole FOV.
        println!("Mismatch sensitivity image and output image size:");
        println!("Sensitivity image: {:3} x {:3} x {:3} pixels, {:3} x {:3} x {:3} mm", inx,iny,inz, mm_(idx),mm_(idy),mm_(idz));
        println!("     Output image: {:3} x {:3} x {:3} pixels, {:3} x {:3} x {:3} mm", enx,eny,enz, mm_(edx),mm_(edy),mm_(edz));
        panic!("For now, the sensitivity image must match the dimensions of the output image exactly.");
    }
}
