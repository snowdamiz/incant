//! Explicit host diagnostic. Production construction continues to use Capture.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MetalArchiveDiagnosticError {
    #[error("strict Metal archive admission refused: {0:?}")]
    Admission(CacheRead),
    #[error("strict Metal archive initialization refused: {0:?}")]
    Native(native::Error),
    #[error("strict Metal archive pipeline contract refused: {0:?}")]
    Refused(Box<native::Counts>),
    #[error("strict Metal archive device error: {0}")]
    Device(String),
}

fn required_input(
    input: Option<PathBuf>,
    read: &CacheRead,
) -> Result<PathBuf, MetalArchiveDiagnosticError> {
    if matches!(read, CacheRead::Supplied { bytes: 1.. })
        && let Some(input) = input
    {
        return Ok(input);
    }
    Err(MetalArchiveDiagnosticError::Admission(read.clone()))
}
fn check_counts(counts: &native::Counts) -> Result<(), MetalArchiveDiagnosticError> {
    let attached = counts.render_attached + counts.compute_attached;
    if counts.disabled
        || counts.disabled_require_refusals != 0
        || counts.render_added != 0
        || counts.compute_added != 0
        || counts.fallback_attempts != 0
        || counts.addition_failures != 0
        || counts.serialization_failures != 0
        || counts.require_failed != 0
        || attached == 0
        || counts.require_succeeded != attached
    {
        return Err(MetalArchiveDiagnosticError::Refused(Box::new(
            counts.clone(),
        )));
    }
    Ok(())
}
impl Session {
    pub(crate) fn require_hits(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
    ) -> Result<(Self, CacheRead), MetalArchiveDiagnosticError> {
        let directory = application_directory().ok_or(MetalArchiveDiagnosticError::Admission(
            CacheRead::NoDirectory,
        ))?;
        // SAFETY: scoped backend borrow; ownership remains with the wgpu device.
        let hal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.ok_or(
            MetalArchiveDiagnosticError::Admission(CacheRead::Unsupported),
        )?;
        let store = Store::open_with_create(
            &directory,
            identity(&adapter.get_info(), &hal.binary_archive_identity()),
            false,
        )
        .map_err(|error| {
            MetalArchiveDiagnosticError::Admission(if error.kind() == io::ErrorKind::NotFound {
                CacheRead::Missing
            } else {
                CacheRead::Io(error.kind())
            })
        })?;
        let (input, read) = store.load();
        let input = required_input(input, &read)?;
        // SAFETY: the same complete owner/ACL/URL and envelope admission used by
        // production. This constructor never accepts a project path or raw bytes.
        let archive = unsafe { hal.begin_binary_archive(Some(&input), Mode::RequireHit) }
            .map_err(MetalArchiveDiagnosticError::Native)?;
        if let Err(error) = store.directory.validate() {
            archive.disable();
            return Err(MetalArchiveDiagnosticError::Admission(CacheRead::Io(
                error.kind(),
            )));
        }
        // Native archive misses are wgpu Internal errors. Capture them on this
        // dedicated diagnostic device so the unchanged cold completion hook can
        // return a typed error, including late optional catalog misses. Ordinary
        // production devices keep their existing error scopes/handler behavior.
        let errors = Arc::new(std::sync::Mutex::new(None));
        let recorded = Arc::clone(&errors);
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            let mut first = recorded.lock().unwrap_or_else(|poison| poison.into_inner());
            if first.is_none() {
                *first = Some(error.to_string());
            }
        }));
        Ok((
            Self {
                archive: Some(archive),
                // No destination exists in a strict session: the shared successful
                // cold completion hook can report counts but cannot serialize/save.
                store: None,
                report: MetalArchiveReport::default(),
                strict_errors: Some(errors),
                // Retain the admitted URL ancestry without a publication store.
                strict_directory: Some(store.directory),
            },
            read,
        ))
    }
    pub(crate) fn check_strict_completion(&self) -> Result<(), MetalArchiveDiagnosticError> {
        let Some(errors) = &self.strict_errors else {
            return Ok(());
        };
        self.strict_directory
            .as_ref()
            .ok_or(MetalArchiveDiagnosticError::Admission(CacheRead::Disabled))?
            .validate()
            .map_err(|error| MetalArchiveDiagnosticError::Admission(CacheRead::Io(error.kind())))?;
        let counts = self
            .archive
            .as_ref()
            .ok_or(MetalArchiveDiagnosticError::Admission(CacheRead::Disabled))?
            .counts();
        check_counts(&counts)?;
        if let Some(error) = errors
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .as_ref()
        {
            return Err(MetalArchiveDiagnosticError::Device(error.clone()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_completion_refuses_capture_fallback_disabled_and_missing_membership() {
        let valid = native::Counts {
            render_attached: 3,
            compute_attached: 2,
            require_succeeded: 5,
            ..Default::default()
        };
        assert!(check_counts(&valid).is_ok());
        for alter in [
            |c: &mut native::Counts| c.render_added = 1,
            |c: &mut native::Counts| c.compute_added = 1,
            |c: &mut native::Counts| c.fallback_attempts = 1,
            |c: &mut native::Counts| c.disabled = true,
            |c: &mut native::Counts| c.disabled_require_refusals = 1,
            |c: &mut native::Counts| c.require_failed = 1,
            |c: &mut native::Counts| c.require_succeeded -= 1,
        ] {
            let mut counts = valid.clone();
            alter(&mut counts);
            assert!(matches!(
                check_counts(&counts),
                Err(MetalArchiveDiagnosticError::Refused(_))
            ));
        }
        assert!(check_counts(&native::Counts::default()).is_err());
    }
    #[test]
    fn strict_input_never_converts_a_missing_or_rejected_cache_into_capture() {
        for read in [
            CacheRead::Missing,
            CacheRead::Rejected,
            CacheRead::Disabled,
            CacheRead::Unsupported,
            CacheRead::NoDirectory,
            CacheRead::Io(io::ErrorKind::PermissionDenied),
        ] {
            assert_eq!(
                required_input(None, &read),
                Err(MetalArchiveDiagnosticError::Admission(read.clone()))
            );
            assert!(required_input(Some(PathBuf::from("/host-owned/fixture")), &read).is_err());
        }
        let read = CacheRead::Supplied { bytes: 4 };
        assert!(required_input(None, &read).is_err());
        assert_eq!(
            required_input(Some(PathBuf::from("/host-owned/fixture")), &read).unwrap(),
            PathBuf::from("/host-owned/fixture")
        );
    }
}
