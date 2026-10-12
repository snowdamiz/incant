use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const PROJECT: &str = "00000000000000000000000001";
const OTHER: &str = "00000000000000000000000002";
fn open(directory: &Path) -> Ledger {
    Ledger::open_at(directory, PROJECT).unwrap()
}
fn spent(lease: Lease, used: u64, uncertain: bool) -> LedgerSnapshot {
    let mut budget = lease.budget();
    budget.used_tokens = used;
    lease.complete(&budget, uncertain).unwrap()
}
fn path(directory: &Path) -> PathBuf {
    directory.join(format!("{PROJECT}.json"))
}

#[test]
fn completed_charges_survive_reopen_and_new_allowances() {
    let temp = tempfile::tempdir().unwrap();
    let first = open(temp.path()).reserve(100).unwrap();
    assert_eq!(first.budget().used_tokens, 0);
    let snapshot = spent(first, 31, false);
    assert_eq!(snapshot.budget.remaining(), 69);
    assert!(!snapshot.accounting_uncertain);
    let reopened = open(temp.path());
    assert_eq!(reopened.snapshot(100).unwrap().budget.used_tokens, 31);
    let second = reopened.reserve(200).unwrap();
    assert_eq!(second.budget().used_tokens, 31);
    spent(second, 80, false);
    let snapshot = open(temp.path()).snapshot(50).unwrap();
    assert_eq!(snapshot.budget.used_tokens, 80);
    assert_eq!(snapshot.budget.remaining(), 0);
    assert!(!snapshot.accounting_uncertain);
    assert!(!snapshot.recovered_reservation);
}

#[test]
fn reservation_is_on_disk_before_lease_is_returned_and_drop_keeps_it() {
    let temp = tempfile::tempdir().unwrap();
    spent(open(temp.path()).reserve(100).unwrap(), 12, false);
    let lease = open(temp.path()).reserve(100).unwrap();
    let record: Record = serde_json::from_slice(&fs::read(path(temp.path())).unwrap()).unwrap();
    assert_eq!(record.charged_tokens, 100);
    assert!(record.accounting_uncertain);
    assert_eq!(record.pending.unwrap().initial_tokens, 12);
    drop(lease);
    let reopened = open(temp.path());
    let snapshot = reopened.snapshot(100).unwrap();
    assert_eq!(snapshot.budget.remaining(), 0);
    assert!(snapshot.accounting_uncertain && snapshot.recovered_reservation);
    assert!(matches!(reopened.reserve(100), Err(LedgerError::Exhausted)));
    // A deliberate higher cap allows more work but does not erase uncertainty.
    let snapshot = spent(open(temp.path()).reserve(200).unwrap(), 123, false);
    assert_eq!(snapshot.budget.used_tokens, 123);
    assert!(snapshot.accounting_uncertain);
    assert!(!snapshot.recovered_reservation);
}

#[test]
fn exact_zero_completion_refunds_but_uncertain_receipts_remain_uncertain() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = spent(open(temp.path()).reserve(100).unwrap(), 0, false);
    assert_eq!(snapshot.budget.remaining(), 100);
    assert!(!snapshot.accounting_uncertain);
    spent(open(temp.path()).reserve(100).unwrap(), 40, true);
    let snapshot = spent(open(temp.path()).reserve(100).unwrap(), 45, false);
    assert_eq!(snapshot.budget.used_tokens, 45);
    assert!(snapshot.accounting_uncertain);
}

#[test]
fn inconsistent_receipt_never_refunds_the_reservation() {
    for bad in [
        Budget {
            max_tokens: 101,
            used_tokens: 20,
        },
        Budget {
            max_tokens: 100,
            used_tokens: 19,
        },
    ] {
        let temp = tempfile::tempdir().unwrap();
        spent(open(temp.path()).reserve(100).unwrap(), 20, false);
        let lease = open(temp.path()).reserve(100).unwrap();
        assert!(matches!(
            lease.complete(&bad, false),
            Err(LedgerError::InvalidReceipt)
        ));
        let snapshot = open(temp.path()).snapshot(100).unwrap();
        assert_eq!(snapshot.budget.used_tokens, 100);
        assert!(snapshot.accounting_uncertain && snapshot.recovered_reservation);
    }
}

#[test]
fn usage_over_the_requested_allowance_is_not_forgiven() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = spent(open(temp.path()).reserve(100).unwrap(), 150, false);
    assert_eq!(snapshot.budget.used_tokens, 150);
    assert_eq!(snapshot.budget.remaining(), 0);
    assert!(!snapshot.accounting_uncertain);
    let snapshot = spent(open(temp.path()).reserve(200).unwrap(), u64::MAX, false);
    assert_eq!(snapshot.budget.used_tokens, MAX_TOKEN_CAP);
    assert!(snapshot.accounting_uncertain);
    assert!(matches!(
        open(temp.path()).reserve(MAX_TOKEN_CAP),
        Err(LedgerError::Exhausted)
    ));
}

#[test]
fn disabled_invalid_or_exhausted_caps_do_not_create_a_receipt() {
    let temp = tempfile::tempdir().unwrap();
    assert!(matches!(
        open(temp.path()).reserve(0),
        Err(LedgerError::Exhausted)
    ));
    assert!(matches!(
        open(temp.path()).reserve(MAX_TOKEN_CAP + 1),
        Err(LedgerError::InvalidRequest)
    ));
    assert!(!path(temp.path()).exists());
    spent(open(temp.path()).reserve(100).unwrap(), 100, false);
    let before = fs::read(path(temp.path())).unwrap();
    assert!(matches!(
        open(temp.path()).reserve(99),
        Err(LedgerError::Exhausted)
    ));
    assert_eq!(fs::read(path(temp.path())).unwrap(), before);
}

#[test]
fn project_identity_is_bounded_and_cannot_escape_directory() {
    let temp = tempfile::tempdir().unwrap();
    let absent = temp.path().join("absent");
    for id in [
        "",
        "../other",
        "0000000000000000000000000I",
        "80000000000000000000000000",
        "0000000000000000000000000a",
    ] {
        assert!(matches!(
            Ledger::open_at(&absent, id),
            Err(LedgerError::InvalidRequest)
        ));
    }
    assert!(!absent.exists());
}

#[test]
fn different_projects_have_independent_locks_and_usage() {
    let temp = tempfile::tempdir().unwrap();
    let first = open(temp.path()).reserve(100).unwrap();
    assert!(matches!(
        Ledger::open_at(temp.path(), PROJECT),
        Err(LedgerError::Busy)
    ));
    let other = Ledger::open_at(temp.path(), OTHER)
        .unwrap()
        .reserve(100)
        .unwrap();
    spent(other, 50, false);
    spent(first, 10, false);
    assert_eq!(
        open(temp.path()).snapshot(100).unwrap().budget.used_tokens,
        10
    );
    assert_eq!(
        Ledger::open_at(temp.path(), OTHER)
            .unwrap()
            .snapshot(100)
            .unwrap()
            .budget
            .used_tokens,
        50
    );
}

#[test]
fn damaged_unknown_or_oversized_record_is_retained_without_reset() {
    let temp = tempfile::tempdir().unwrap();
    spent(open(temp.path()).reserve(100).unwrap(), 10, false);
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(path(temp.path())).unwrap()).unwrap();
    let cases = [
        serde_json::json!({"version": 2}),
        serde_json::json!({"project_id": OTHER}),
        serde_json::json!({"charged_tokens": MAX_TOKEN_CAP + 1}),
        serde_json::json!({"charged_tokens": -1}),
        serde_json::json!({"extra": 1}),
        serde_json::json!({"pending": {"initial_tokens": 10, "max_tokens": 10, "prior_uncertain": false}}),
        serde_json::json!({"pending": {"initial_tokens": 1, "max_tokens": 20, "prior_uncertain": false}}),
        serde_json::json!({"pending": {"initial_tokens": 1, "max_tokens": 10, "prior_uncertain": false}}),
    ];
    let mut records = vec![
        b"not JSON".to_vec(),
        vec![b' '; MAX_RECORD_BYTES as usize + 1],
    ];
    for patch in cases {
        let mut changed = original.clone();
        changed
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        records.push(serde_json::to_vec(&changed).unwrap());
    }
    for bytes in records {
        fs::write(path(temp.path()), &bytes).unwrap();
        assert!(matches!(
            Ledger::open_at(temp.path(), PROJECT),
            Err(LedgerError::InvalidRecord)
        ));
        assert_eq!(fs::read(path(temp.path())).unwrap(), bytes);
    }
}

#[cfg(unix)]
#[test]
fn files_are_private_and_links_are_refused() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("usage");
    spent(open(&directory).reserve(100).unwrap(), 10, false);
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(path(&directory)).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let alias = temp.path().join("alias");
    symlink(&directory, &alias).unwrap();
    assert!(matches!(
        Ledger::open_at(&alias, PROJECT),
        Err(LedgerError::Storage)
    ));
    let outside = temp.path().join("outside");
    fs::write(&outside, b"unchanged").unwrap();
    fs::remove_file(path(&directory)).unwrap();
    symlink(&outside, path(&directory)).unwrap();
    assert!(matches!(
        Ledger::open_at(&directory, PROJECT),
        Err(LedgerError::Storage)
    ));
    fs::remove_file(path(&directory)).unwrap();
    let lock = directory.join(format!("{PROJECT}.lock"));
    fs::remove_file(&lock).unwrap();
    symlink(&outside, &lock).unwrap();
    assert!(matches!(
        Ledger::open_at(&directory, PROJECT),
        Err(LedgerError::Storage)
    ));
    assert_eq!(fs::read(&outside).unwrap(), b"unchanged");
}

fn child(directory: &Path, mode: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ledger::tests::subprocess_worker", "--nocapture"])
        .env("INCANT_LEDGER_TEST_DIRECTORY", directory)
        .env("INCANT_LEDGER_TEST_MODE", mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap()
}
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn subprocess_worker() {
    let Some(directory) = std::env::var_os("INCANT_LEDGER_TEST_DIRECTORY") else {
        return;
    };
    let directory = Path::new(&directory);
    let lease = open(directory).reserve(100).unwrap();
    match std::env::var("INCANT_LEDGER_TEST_MODE").unwrap().as_str() {
        "crash" => std::process::exit(29),
        "hold" => {
            fs::write(directory.join("ready"), b"ready").unwrap();
            let mut input = [0_u8; 1];
            std::io::stdin().read_exact(&mut input).unwrap();
            spent(lease, 37, false);
        }
        _ => panic!("unknown test mode"),
    }
}
#[test]
fn fresh_process_abrupt_exit_preserves_reservation() {
    let temp = tempfile::tempdir().unwrap();
    let mut process = OwnedChild(child(temp.path(), "crash"));
    assert_eq!(process.0.wait().unwrap().code(), Some(29));
    let snapshot = open(temp.path()).snapshot(100).unwrap();
    assert_eq!(snapshot.budget.used_tokens, 100);
    assert!(snapshot.accounting_uncertain && snapshot.recovered_reservation);
}
#[test]
fn live_process_exclusion_then_completed_receipt_survives_exit() {
    let temp = tempfile::tempdir().unwrap();
    let mut process = OwnedChild(child(temp.path(), "hold"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !temp.path().join("ready").exists() {
        assert!(Instant::now() < deadline, "child did not reach reservation");
        assert!(
            process.0.try_wait().unwrap().is_none(),
            "child exited before ready"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(
        Ledger::open_at(temp.path(), PROJECT),
        Err(LedgerError::Busy)
    ));
    process.0.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(process.0.wait().unwrap().success());
    let snapshot = open(temp.path()).snapshot(100).unwrap();
    assert_eq!(snapshot.budget.used_tokens, 37);
    assert!(!snapshot.accounting_uncertain && !snapshot.recovered_reservation);
}
