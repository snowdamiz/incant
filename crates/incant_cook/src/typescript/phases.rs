//! Bounded, allowlisted observations from the private compiler process.
use std::{fs::File, io::Read, path::Path, time::Duration};

pub(super) const MAX_BYTES: u64 = 4096;
const PHASES: [&str; 10] = [
    "driver_entry",
    "typescript_ready",
    "swc_ready",
    "compiler_ready",
    "input_ready",
    "snapshot_ready",
    "typecheck_done",
    "transform_done",
    "artifact_ready",
    "publish_done",
];

pub(super) fn observation(path: &Path, elapsed: Duration, outcome: &str) -> serde_json::Value {
    let mut bytes = Vec::new();
    let parsed = File::open(path)
        .and_then(|file| file.take(MAX_BYTES + 1).read_to_end(&mut bytes))
        .ok()
        .and_then(|_| parse(&bytes));
    let (state, phases) = match parsed {
        Some(value) => value,
        None => ("invalid_or_unavailable", Vec::new()),
    };
    serde_json::json!({
        "version": 2,
        "outcome": outcome,
        "parent_elapsed_us": u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
        "state": state,
        "phases": phases,
    })
}

fn parse(bytes: &[u8]) -> Option<(&'static str, Vec<serde_json::Value>)> {
    if bytes.len() > MAX_BYTES as usize {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    // A killed child may leave a partial final write. Retain only complete rows.
    let end = text.rfind('\n').map_or(0, |index| index + 1);
    let mut phases = Vec::new();
    let mut previous = 0;
    for line in text[..end].lines() {
        let row: serde_json::Value = serde_json::from_str(line).ok()?;
        let object = row.as_object()?;
        let name = *PHASES.get(phases.len())?;
        let elapsed = row.get("elapsed_us")?.as_u64()?;
        if object.len() != 2
            || row.get("phase")?.as_str()? != name
            || elapsed < previous
            || elapsed > 86_400_000_000
        {
            return None;
        }
        // Reconstruct from known literals; never emit arbitrary child content.
        phases.push(serde_json::json!({ "phase": name, "elapsed_us": elapsed }));
        previous = elapsed;
    }
    let state = if bytes.is_empty() {
        "empty"
    } else if end != text.len() {
        "partial_final_row"
    } else {
        "valid_prefix"
    };
    Some((state, phases))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_phase_prefix_survives_partial_write_without_exposing_tail() {
        let bytes = b"{\"phase\":\"driver_entry\",\"elapsed_us\":1}\n{private-unfinished";
        let (state, rows) = parse(bytes).unwrap();
        assert_eq!(state, "partial_final_row");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["phase"], "driver_entry");
        assert!(!serde_json::to_string(&rows).unwrap().contains("private"));
        assert_eq!(parse(b"").unwrap(), ("empty", Vec::new()));
    }

    #[test]
    fn loader_deadlines_retain_the_last_completed_dependency() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut prefix = String::new();
        for (index, name) in PHASES[..4].iter().enumerate() {
            prefix.push_str(&format!(
                "{{\"phase\":\"{name}\",\"elapsed_us\":{index}}}\n"
            ));
            std::fs::write(file.path(), &prefix).unwrap();
            let observed = observation(file.path(), Duration::from_secs(8), "deadline");
            assert_eq!(observed["version"], 2);
            assert_eq!(observed["outcome"], "deadline");
            assert_eq!(observed["state"], "valid_prefix");
            assert_eq!(observed["phases"].as_array().unwrap().len(), index + 1);
            assert_eq!(observed["phases"][index]["phase"], *name);
        }
    }

    #[test]
    fn observations_reject_unknown_fields_names_order_times_and_excess_bytes() {
        for invalid in [
            "{\"phase\":\"private-source\",\"elapsed_us\":0}\n",
            "{\"phase\":\"driver_entry\",\"elapsed_us\":0,\"source\":\"private\"}\n",
            "{\"phase\":\"compiler_ready\",\"elapsed_us\":0}\n",
            "{\"phase\":\"driver_entry\",\"elapsed_us\":-1}\n",
            "{\"phase\":\"driver_entry\",\"elapsed_us\":0.5}\n",
            "{\"phase\":\"driver_entry\",\"elapsed_us\":86400000001}\n",
            "{\"phase\":\"driver_entry\",\"elapsed_us\":2}\n{\"phase\":\"compiler_ready\",\"elapsed_us\":1}\n",
        ] {
            assert!(parse(invalid.as_bytes()).is_none());
        }
        assert!(parse(&vec![b' '; MAX_BYTES as usize + 1]).is_none());
        let all = PHASES
            .iter()
            .enumerate()
            .map(|(index, name)| format!("{{\"phase\":\"{name}\",\"elapsed_us\":{index}}}\n"))
            .collect::<String>();
        assert_eq!(parse(all.as_bytes()).unwrap().1.len(), PHASES.len());
        assert!(
            parse(format!("{all}{{\"phase\":\"publish_done\",\"elapsed_us\":9}}\n").as_bytes())
                .is_none()
        );
    }
}
