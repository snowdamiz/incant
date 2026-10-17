# Release checkpoint50

[PR374](https://github.com/snowdamiz/incant/pull/374) promoted selected main376
`04a30f70d7bdcf090077992b4d1f704fb4727a05` after all 23 required contexts and all
eight push workflows reported actual success on frozen candidate
`06113baf2e8a02758b74fe3e6863dd8cf0771cea`.

The October 17 release merge is `7e878a411897a769ac7f85dfd96c15029a23e3d7`.
Its tree `d828669e90faa55007eb66ba12863128c1d4bef4` equals both the tested
candidate and selected main. The merge preserves existing history and includes
`Built-by: astra`. [Exact checks, workflows and identities](evidence/release-checkpoint-50.json).

This baseline adds explicit-state TypeScript coroutines, native Wasm restart
ownership, sampled Core locale qualification and complete host-stage observation.
It also verifies the normal headless dependency repair that blocked checkpoint49.
The original failed batch remains historical. The website deployment job is
outside the required contexts and stayed skipped; its build and workflow passed.

Main changes after PR376 remain outside this tested release tree. In particular,
later Wasm export, wake/acquisition observation and native material workflow work
retain their own local evidence. No distribution was published. Rendering
quality, complete-game performance and phase gates remain open.
