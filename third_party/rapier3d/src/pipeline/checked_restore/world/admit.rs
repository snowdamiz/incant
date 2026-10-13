//! The epoch and numeric witness can only be derived from the same envelope.
//! This is a private data binding, not capture authentication or a public format.
use super::super::{AdmissionError, WorkBudget, admit_numeric_frame};
use super::{AssemblyFrame, NumericFrame, allocation::*};
use incant_physics_snapshot::{Budget, Error, Identity, with_decoded_world_boundary};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AssemblyError {
    Decode(Error),
    Numeric(AdmissionError),
    Construction(ConstructionError),
}
pub(super) struct AdmittedAssembly<'a> {
    numeric: NumericFrame<'a>,
    impulse_epoch: u32,
}
impl AdmittedAssembly<'_> {
    pub(super) fn numeric(&self) -> &NumericFrame<'_> {
        &self.numeric
    }
    pub(super) fn impulse_epoch(&self) -> u32 {
        self.impulse_epoch
    }
}
pub(super) fn with_admitted<T>(
    assembly: &AssemblyFrame<'_>,
    expected: Identity,
    decode_budget: &mut Budget,
    work_budget: &mut WorkBudget,
    f: impl for<'a> FnOnce(&AdmittedAssembly<'a>) -> ConstructionResult<T>,
) -> core::result::Result<T, AssemblyError> {
    let mut outcome = None;
    with_decoded_world_boundary(assembly.inner(), expected, decode_budget, |frame| {
        outcome = Some(match admit_numeric_frame(frame, expected, work_budget) {
            Ok(numeric) => f(&AdmittedAssembly {
                numeric,
                impulse_epoch: assembly.impulse_epoch(),
            })
            .map_err(AssemblyError::Construction),
            Err(e) => Err(AssemblyError::Numeric(e)),
        });
        Ok(())
    })
    .map_err(AssemblyError::Decode)?;
    outcome.ok_or(AssemblyError::Construction(ConstructionError::Incomplete))?
}
