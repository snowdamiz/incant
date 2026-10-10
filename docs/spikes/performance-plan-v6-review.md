# Revision 6 final update review

Reviewed 2026-10-10 after the director confirmed this is the final plan update
and requested removal of the temporary watcher. The watcher was deleted. This
review supersedes revision 5's hardware-purchase and signing-prerequisite wording;
the native-runtime migration order and all numerical performance budgets remain.

## Adopted changes

- Incant and Driftwake are settled names. Name clearance, domain registration,
  package reservations and fallback names are removed requirements, not pending
  tasks or tasks completed by this review.
- Apple/Google/Steam accounts and Apple/Windows distribution signing move to the
  first Phase 5 store uploads. The Phase 0 ledger records the removal and the
  executable gate no longer requires `signing_certificates_obtained`. It still
  fails on missing spike/nightly evidence, staffing or director approval.
- Use AWS Device Farm hosted real phones, not purchased reference phones. The
  first runs need the director's AWS account/billing. The ten-device expansion
  remains Phase 5 work. No AWS resources, spending or credentials were created.
- Linux CI exercises Deck resolution/input; Valve's review supplies the actual
  Deck compatibility decision. A Linux test alone does not prove device suspend,
  resume or Deck Verified status.

## Device-test implementation constraints

AWS documents iOS **device** builds packaged as IPA, and service re-signing with
a replacement wildcard profile. The selected route therefore relies on AWS
signing the test installation rather than on our distribution certificate. The
first packaged Incant upload/install/run remains an integration check; a local
simulator build or the selection of this route is not that evidence.
See [app preparation and re-signing](https://docs.aws.amazon.com/devicefarm/latest/developerguide/apps.html)
and [XCTest package preparation](https://docs.aws.amazon.com/devicefarm/latest/developerguide/test-types-ios-xctest.html).

Re-signing removes entitlements including Game Center, in-app purchase and push
notifications. These performance test builds cannot prove those store services;
their signed integration checks remain with Phase 5. This does not reintroduce
signing into Phase 0. Device Farm's mobile-device service is in `us-west-2`;
the other selected AWS regions are for the online services. See
[Device Farm setup](https://docs.aws.amazon.com/devicefarm/latest/developerguide/getting-started.html).

For the plan's nearest-available-model fallback, retain actual model, OS, CPU/GPU,
build/profile and test settings in every result. Compare a PR and its baseline
on matching configurations. Keep the 2 ms p95/10,000-entity and 5% regression
thresholds; do not label a substitute as an iPhone 13 or Pixel 6. A changed model
requires its own baseline rather than inheriting another device's timing. Record
queue/availability limitations separately from an engine regression.

## Verification and remaining work

The gate behavior tests cover removed signing requirements and preserve failure
for every still-required evidence/approval category. Convention generation,
Python tool tests, local documentation links and whitespace checks run for this
revision; hosted checks must pass on the final PR head before merge.

No phone performance, nightly history, phase approval or completed runtime
migration is claimed. The active component extraction and Claude Opus 5.5 Max UI
work continue under the updated plan. The Phase 0 review ledger remains open.
