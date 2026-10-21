# 2026-10-21: `cargo xtask ci` hung in the portal save tests

## Symptom
On branch pf-5-14-welt the full CI ran past two hours: the test
`portal_save::tests::the_portal_is_asked_on_the_first_file_type_and_answers_the_one_picked`
(apps/vectorcraft) "has been running for over 60 seconds" and never finished. Earlier runs on the
same code had passed, and the change under test (welt pockets) does not touch the app.

## Reproduce
- The test alone (`patternfoundry-… -q portal_save`), 20 runs: all passed in 0.3 s, with a real
  `dbus-daemon` (not skipped).
- First attempt was wrong: `timeout 60 cargo test …` timed out 20 times out of 20 because the
  timeout covered the build too. Built once with `--no-run`, then timed the test binary only.
- The whole app test binary (130 tests at once), 15 runs: 2 hung, one in each of the two D-Bus
  tests (`…answers_the_one_picked` and `the_answer_is_awaited_on_the_path_the_portal_returns`).
  So it needs other tests competing for the CPU.

## Leads
- The client listens for `Response` before calling `SaveFile` (zbus' `receive_signal` adds its
  match rule before returning), so a lost answer on the client side was unlikely for the first
  test.
- The mock portal made its `MessageIterator` inside the thread it spawns. zbus drops incoming
  messages while no stream listens on a connection, so when the thread was not yet running the
  client's `SaveFile` call was dropped and `chooser.call` waited forever (no timeout). This fits
  both tests and the need for load.
- Also seen, not changed: for a portal older than 0.9 the client listens on the returned request
  path only after `SaveFile` returns; the mock waits 300 ms before answering there. Under heavy
  load that is a second, smaller window (no hang seen after the fix).

## Fix and check
The mock's `MessageIterator` is made before the thread starts (before the client exists).
- With the fix, the whole binary: 0 hangs in 80 runs (two batches of 40).
- Fix undone: 1 hang in 30 runs; re-applied: 0 in 40.
