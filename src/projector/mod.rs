pub use siddon::Siddon;

pub mod siddon;

/// Abstract interface for forward-backward projection implementations
pub trait Projector {
    fn project_one_lor<'i, 'g>(fold_state: FoldState<'i, 'g>, lor: &LOR) -> FoldState<'i, 'g>;

    // Sparse storage of the slice through the system matrix which corresponds
    // to the current LOR. Allocating these anew for each LOR had a noticeable
    // runtime cost, so we create them up-front and reuse them.
    // This should probably have a default implementation
    fn buffers(fov: FOV) -> SystemMatrixRow;
}


// Data needed by project_one_lor, both as input and output, because of the
// constrains imposed by `fold`
pub type FoldState<'i, 'g> = (ImageData, SystemMatrixRow, &'i Image, &'g Option<Gaussian>);

use crate::{
    LOR,
    fov::FOV,
    gauss::Gaussian,
    image::{ImageData, Image},
    system_matrix::SystemMatrixRow,
};
