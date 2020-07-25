use crate::config::{Config, ValidationOpts};
use crate::ValidateCmdOpts;
use anyhow::Result;

/// The validate sub command allows for checking any given configuration file without
/// rendering to disk.
pub struct ValidateCmd {
    opts: ValidateCmdOpts,
}

impl ValidateCmd {
    /// Create sub command struct to run validation of the given input file
    pub fn new(opts: ValidateCmdOpts) -> Self {
        Self { opts }
    }

    /// Main entry point to run the validator
    /// will return nothing on the happy path and descriptive errors on failure
    pub fn run(&self) -> Result<()> {
        debug!("validation options: {:?}", self.opts);

        for file in &self.opts.input_files {
            Config::load(&file)?.validate(&ValidationOpts::default())?;
        }

        Ok(())
    }
}
