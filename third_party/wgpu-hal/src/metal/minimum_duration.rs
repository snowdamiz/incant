//! Explicit host-owned diagnostic. This does not change the simulation scheduler.
pub use super::minimum_duration_policy::{Error, Policy};

pub fn check_supported() -> Result<(), Error> {
    if objc2::available!(macos = 10.15.4) {
        Ok(())
    } else {
        Err(Error::Unsupported)
    }
}

impl super::Surface {
    /// Recheck the transaction property before configure/acquire, without
    /// changing the sealed policy or acquiring a drawable.
    pub fn check_minimum_duration60(&self) -> Result<(), Error> {
        check_supported()?;
        if self.render_layer.lock().presentsWithTransaction() {
            Err(Error::Transaction)
        } else {
            Ok(())
        }
    }
    /// Select only before first configure, without acquiring or retaining a
    /// drawable. Not a project-controlled setting or a runtime quality toggle.
    pub fn select_minimum_duration60(&self) -> Result<(), Error> {
        check_supported()?;
        let layer = self.render_layer.lock();
        self.minimum_duration
            .select(true, layer.presentsWithTransaction())
    }
}
