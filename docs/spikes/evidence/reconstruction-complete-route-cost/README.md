# Portable E040 cost evidence

Run from any checkout with Python 3.12 or newer, without `-O`:

```sh
python3 docs/spikes/evidence/reconstruction-complete-route-cost/verify.py
```

This is a data-only replay. It starts no child, native renderer, GPU or network
operation, extracts no archive paths, and writes no files. Compressed input is
capped at 1 MiB, expanded tar at 8 MiB, each member at 2 MiB, the record set at
exactly 704 members, and analysis at 120 seconds. All 704 original record hashes
and raw timestamp ticks are checked before reproducing `analysis.json` exactly
apart from its original elapsed analysis time. Archive members are regular files
with exact manifest names; symlinks, unexpected or duplicate entries are refused.

- `records.tar.gz`: exact result/resources and every timing/parity/query record;
  4,551,298 payload bytes, compressed without filename or wall-time metadata.
- `provenance.json`: individual byte lengths/hashes, original absolute origin
  (metadata only), source/artifact/proposal bindings and exact build command.
- `analyze-original.py`: unchanged originally executed analyzer, retained for
  inspection. **Use `verify.py`**, whose narrow transport/entrypoint changes are
  shown in `analyzer-portability.diff`; statistical formulas are unchanged.
- `host.json`, `closeout.json`, `analysis.json`, `triage.json`: exact original
  host, closure and complete reports. Original paths are provenance, not required
  local dependencies.
- `E038-failure.json`, `E038-zero-query.bin`, `E039-failure.json`: exact preceding
  failure records, with all-zero query bytes. Large failed image payloads stay
  locally retained and are not duplicated into Git.

The portable replay validates the timing arithmetic and retained receipts. It
cannot independently rerun released pixel comparisons or establish appearance.
`sample_contracts.exact_all_required=false` and coarse RGB differences in parity
receipts are retained diagnostic observations, not silently converted to passes.
The native route required exact metadata/alpha/repair, and exact G/L same-route
output identity; it did not require all coarse RGB to equal O.

See [the report](../../reconstruction-complete-route-cost.md) for interpretation,
profile limits and the next source-only design decision.
