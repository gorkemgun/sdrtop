# Phase 10 — Configuration & Persistence: Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Steps](Phase%2010%20-%20Configuration%20%26%20Persistence%20-%20Steps.md)

---

## What was built

Radio settings (frequency, gains, amp) and display state (active preset, waterfall row count) now survive application restarts. On startup the app reads `~/.config/sdrtop/config.toml` (or a custom path via `--config`), applies settings to hardware, and on clean exit (`q`) writes the current state back. CLI flags `--frequency`, `--lna`, `--vga` override config file values.

---

## Deviations from the plan

**Per-field serde defaults required.** The plan showed `RadioConfig` fields without `#[serde(default)]` annotations. A partial TOML section like `[radio]\nfrequency_hz = 433000000` (with `sample_rate`, `lna_gain`, `vga_gain` absent) caused serde to fail rather than fill missing fields with defaults. Fix: six per-field default helper functions (`default_frequency_hz`, etc.) annotated with `#[serde(default = "fn_name")]` on each field. The plan's correctness note about `#[serde(default)]` on `AppConfig` fields only handles *missing sections*, not *missing fields within a present section*.

**`let mut engine`** — The plan's `App::new()` snippet showed `let engine = ...` (immutable), but `engine.set_preset(...)` requires a mutable binding. Changed to `let mut engine`.

No other deviations. Steps 1–5 followed in order with no rework beyond the serde fix.

---

## Key decisions

**`$HOME` env var instead of `dirs` crate.** `std::env::var_os("HOME")` is sufficient on Linux without adding a new dependency.

**Config scope.** Roadmap listed `fft_size`, `theme`, `spectrum_db_min/max`, etc. These were deliberately excluded from `AppConfig` — none are user-controllable at runtime yet. Only the five fields that the user can actually change from the TUI (frequency, sample rate, LNA/VGA gains, amp) and the two display fields (active preset, waterfall max rows) are persisted.

**Lock discipline in `save_config()`.** Values are extracted from `SdrMetrics` while holding the mutex, guard dropped, then file I/O happens outside the lock. This is documented in the steps file as a correctness requirement.

---

## Test results

31 unit tests pass, zero clippy warnings, clean release build.

```
running 31 tests
... (all ok)
test result: ok. 31 passed; 0 failed
```

---

## Files changed

| File | Change |
|---|---|
| `Cargo.toml` | Added `clap = { version = "4", features = ["derive"] }` |
| `src/config.rs` | Added `RadioConfig`, `DisplayConfig`, `AppConfig` with `load_or_default()`, `save()`, 4 new unit tests, per-field serde default helpers |
| `src/main.rs` | Full rewrite: `Cli` struct (clap derive), `default_config_path()`, restructured `main()` |
| `src/app.rs` | `App::new(cfg, config_path)` signature, `config_path` field, hardware application at startup, `save_config()` helper, `'q'` calls `save_config()` |
