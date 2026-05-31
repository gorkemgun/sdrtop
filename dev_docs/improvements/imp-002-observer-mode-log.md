# IMP-002 — Observer Mode — Log

← [Home](../Home.md) | [Plan](imp-002-observer-mode.md)

## Status: ✅ Done (alpha)

> **This is an alpha implementation.** It shows everything reachable without opening the device. What's missing: live IQ data, current frequency/gain of the owner app, auto-recovery when the device is freed. See [Phase 16](../Roadmap.md#phase-16--advanced-observer-mode--idea) for the longer-term direction.

---

## Deviations from plan

### `cpu_pct` not stored in `OwnerInfo` — computed in polling task

The plan had `cpu_pct: f32` as a field on `OwnerInfo`. But `OwnerInfo` is built by `sysfs::find_owner()` which reads a single snapshot of `/proc/PID/stat`. Computing CPU% requires two snapshots with a known time delta — it's fundamentally a polling-task concern.

**Decision:** `OwnerInfo` stores raw `cpu_ticks: u64`. The observer polling task in `app.rs` keeps `last_owner_cpu: Option<(u64, Instant)>` across iterations and computes the delta each second. `SdrMetrics` stores `observer_owner_cpu_pct: f32` (the result), not ticks.

### `App::new()` split into `new_normal()` and `new_observer()`

The plan described the fallback logic inline in `App::new()`. The normal mode setup is ~200 lines; adding the observer branch inline would have made `App::new()` unmaintainable. **Decision:** Split into two private associated functions. `App::new()` is a 10-line dispatcher. `spawn_sys_resource_task()` extracted as a shared helper called by both branches.

### `max_power` kept as `String`, not parsed to `u32`

The plan document shows `max_power_ma: u32` in `HackRfSysInfo`. The sysfs file `bMaxPower` contains values like `"500mA"` — a unit suffix, not a bare number. Parsing this reliably requires stripping the suffix. **Decision:** Store as `String` and display as-is. The observer panel shows `"500mA"` directly, which is more legible and correct than a bare integer.

### `obs_bus` / `obs_dev` unused in observer polling task

The plan mentioned using `bus` and `dev` to call `find_owner(bus, dev)` inside the polling task. The live `find_hackrf()` call already returns fresh `bus` and `dev` each tick, so the captured values from startup are redundant. They're suppressed with `let _ = (obs_bus, obs_dev)`. A future improvement could use them to detect if the device was disconnected and reconnected on a different port.

---

## Bug found during implementation: `?` in `find_hackrf()` exits function instead of continuing loop

**Symptom:** Observer mode never activated. Device open failed, sysfs check returned `None`, app exited with "hackrf is busy" error instead of entering observer mode.

**Root cause:** In `sysfs::find_hackrf()`:

```rust
// BEFORE (bug):
let vid = read_sysfs(&base.join("idVendor"))?;
```

The `?` operator propagates `None` out of the **entire function**, not just the current loop iteration. `/sys/bus/usb/devices/` contains interface entries (e.g. `1-1:1.0`) that have no `idVendor` file. The first such entry caused `find_hackrf()` to return `None` immediately, before reaching the HackRF entry.

**Fix:**

```rust
// AFTER:
let vid = match read_sysfs(&base.join("idVendor")) {
    Some(v) => v,
    None => continue, // interface entries (X:Y.Z) don't have idVendor
};
```

The `pid` lookup below correctly used `match ... continue` — only `vid` had this bug.

**Why it wasn't caught in earlier testing:** `find_hackrf()` is only called when `Device::open()` fails. During development the device was always free, so this code path was never exercised.

---

## Key decisions

**`find_hackrf()` is the gate, not the error code** — observer mode triggers when the device is physically present (sysfs confirms VID/PID) but libhackrf can't open it. The specific libhackrf error code is irrelevant: any failure + sysfs-present = observer mode. This keeps the condition simple and handles all "busy" variants (exclusive USB, udev permissions, etc.) uniformly.

**Hardware keys use match guards, not inner `if` blocks** — clippy suggested collapsing `KeyCode::Char(' ') => { if self.device.is_some() { ... } }` into `KeyCode::Char(' ') if self.device.is_some() => { ... }`. Applied across all hardware-gated keys (`Space`, `F`, `S`). The `if let Some(device)` pattern is retained for keys that need the device reference (`R`, arrows, `[`, `]`, `A`).

**Observer preset auto-applied, not user-persisted** — `save_config()` is a no-op in observer mode (`self.device.is_none()` guard). The observer preset is set at startup and overrides whatever preset was last saved. The user can switch presets manually, but they won't be persisted. This avoids the config file being written with "observer" as the active preset when the device becomes free again.

**`fmt_duration` is module-private in `app.rs`** — the plan had no opinion on location. It's only needed by the observer polling task and its output is stored as `Option<String>` in state, so `observer.rs` never needs it. Kept in `app.rs` as a private free function.

---

## Files changed

| File | Change |
|---|---|
| `src/hardware/mod.rs` | Exported `pub mod sysfs` |
| `src/hardware/sysfs.rs` | Bug fix: `?` → `match ... continue` in `find_hackrf()` |
| `src/state.rs` | Added 10 observer fields to `SdrMetrics` |
| `src/config.rs` | Added `"observer"` preset |
| `src/ui/observer.rs` | New — `ObserverPanel` |
| `src/ui/mod.rs` | Added `observer` module and `ObserverPanel` re-export |
| `src/ui/footer.rs` | Observer mode arm: "Observer Mode — Hardware controls disabled. [Q] Quit [?] Help" |
| `src/app.rs` | Split `new()` into `new_normal()` / `new_observer()`; `device` and `rx_ctx` are now `Option<Arc<...>>`; observer sysfs polling task; `spawn_sys_resource_task()` helper; hardware key guards; `save_config()` no-op in observer mode; `fmt_duration()` helper; 1 new test |

---

## Test results

```
running 43 tests
... all pass
cargo clippy  →  Finished (no warnings)
```
