use crate::config::{Config, ValidationOpts};
use crate::RenderCmdOpts;
use anyhow::Result;

pub struct RenderCmd {
    opts: RenderCmdOpts,
}

impl RenderCmd {
    /// Create sub command struct to render deployments of the given input file(s)
    pub fn new(opts: RenderCmdOpts) -> Self {
        Self { opts }
    }

    /// Main entry point to run the rendering process
    /// will return nothing on the happy path and descriptive errors on failure
    pub fn run(&mut self) -> Result<()> {
        debug!("render options: {:?}", self.opts);

        for file in &self.opts.input_files {
            Config::load(&file)?.validate(&ValidationOpts::default())?;
        }

        Ok(())
    }
}
