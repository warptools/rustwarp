use warpforge_api::plot::PlotCapsule;
use warpforge_terminal::logln;

use crate::context::Context;
use crate::Result;

pub async fn run_plot(plot: PlotCapsule, _context: &Context) -> Result<()> {
	logln!("{plot:#?}");

	Ok(())
}
