# Progress

- 2026-11-06 (issue 121): removed `/crates/affinity/fuzz` from the Dependabot cargo directories. It is a standalone workspace with a path dependency on its parent crate, which Dependabot cannot resolve alone, so the job failed every run. Config-only change; no Rust touched.
