# IMP-002 — Observer Mode (HackRF in use by another app)

← [Home](../Home.md)

**Status:** 📋 Planned  
**Between phases:** 11 → 12  

---

## Goal

When another app (e.g. SDR++) already has the HackRF open, sdrtop should not crash. Instead it enters **observer mode**: it shows everything it can without exclusive device access, and clearly communicates what it can and cannot do.

---

## The fundamental constraint

libhackrf uses exclusive USB access. If SDR++ has the device open, `hackrf_open()` fails — there is no way to share the IQ stream or read gain/frequency settings from the hardware. This is a USB-level constraint, not a software limitation of sdrtop.

---

## What IS available without opening the device

### From `/sys/bus/usb/devices/X-Y/` (no root needed)

| File | Data |
|---|---|
| `product` | "HackRF One" |
| `manufacturer` | "Great Scott Gadgets" |
| `serial` | Full serial number (e.g. `a7b4c3d100000000`) |
| `speed` | "480" → USB High Speed |
| `bMaxPower` | Max current draw (e.g. "500mA") |
| `version` | USB version ("2.00") |
| `busnum`, `devnum` | Bus and port → device node path |
| `power/connected_duration` | Microseconds since device was connected |
| `power/active_duration` | Microseconds device was not suspended |

### From `/proc/PID/` (readable if same user — the common case)

The device node is `/dev/bus/usb/BUS/DEV`. We scan `/proc/*/fd/` symlinks to find
which process has that node open. For processes owned by a different user, we skip
gracefully. If sdrtop and SDR++ run as the same user (typical), this always works.

| File | Data |
|---|---|
| `comm` | Short process name ("sdrpp") |
| `cmdline` | Full command line with arguments |
| `stat` | CPU ticks + starttime → CPU% and running time |
| `status` | VmRSS → memory usage in MB |

### Not available (even with root)

- Current frequency, gain settings (live state inside the owner process)
- Raw IQ stream (exclusive USB — would need usbmon + root + custom parsing)

---

## What the observer panel shows

```
┌─ Observer Mode ───────────────────────────────────────────────┐
│                                                               │
│  HackRF One · Great Scott Gadgets                             │
│  Serial: a7b4c3d100000000                                     │
│  USB High Speed (480 Mbit/s) · 500 mA · Bus 1, Port 3        │
│  Connected: 1h 23m 14s                                        │
│                                                               │
│  In use by: sdrpp  (PID 12345)                                │
│  /usr/bin/sdrpp --config /home/user/.config/sdrpp             │
│  CPU: 12.3%  ·  RAM: 145 MB  ·  Running: 23m 14s             │
│                                                               │
│  Hardware controls disabled.                                  │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

System resources and log panels remain fully functional alongside this.

---

## Architecture

### New: `src/hardware/sysfs.rs`

Pure filesystem reads, no libhackrf involved.

```rust
pub struct HackRfSysInfo {
    pub product: String,         // "HackRF One"
    pub manufacturer: String,    // "Great Scott Gadgets"
    pub serial: String,          // "a7b4c3d100000000"
    pub speed_mbits: u32,        // 480
    pub max_power_ma: u32,       // 500
    pub bus: u32,
    pub dev: u32,
    pub connected_secs: Option<u64>,
}

pub struct OwnerInfo {
    pub pid: u32,
    pub name: String,            // from /proc/PID/comm
    pub cmdline: String,         // from /proc/PID/cmdline
    pub cpu_pct: f32,            // calculated from /proc/PID/stat
    pub rss_mb: u64,             // from /proc/PID/status
    pub running_secs: u64,       // from /proc/PID/stat starttime + uptime
}

/// Scans /sys/bus/usb/devices/ for VID=1d50 PID=6089
pub fn find_hackrf() -> Option<HackRfSysInfo>

/// Scans /proc/*/fd/ for the device node /dev/bus/usb/BUS/DEV
pub fn find_owner(bus: u32, dev: u32) -> Option<OwnerInfo>
```

### Changes: `src/hardware/mod.rs`

Export the new `sysfs` module.

### Changes: `src/state.rs`

```rust
pub observer_mode: bool,
pub observer_device: Option<String>,   // "HackRF One · Great Scott Gadgets"
pub observer_serial: Option<String>,   // "a7b4c3d100000000"
pub observer_usb: Option<String>,      // "High Speed (480 Mbit/s) · 500 mA · Bus 1, Port 3"
pub observer_connected: Option<String>,// "1h 23m 14s"
pub observer_owner: Option<String>,    // "sdrpp (PID 12345)"
pub observer_cmdline: Option<String>,  // full command line
pub observer_owner_cpu: f32,
pub observer_owner_ram_mb: u64,
pub observer_owner_uptime: Option<String>, // "23m 14s"
```

### Changes: `src/app.rs`

**`App` struct:**
```rust
device: Option<Arc<hardware::Device>>,  // None in observer mode
rx_ctx: Option<Arc<RxContext>>,         // None in observer mode
```

**`App::new()` fallback logic:**
1. Try `Device::open()`
2. Success → normal mode (no change to existing behaviour)
3. Failure → check `sysfs::find_hackrf()`
   - Found → observer mode (`device = None`, no hardware init)
   - Not found → bail with "No HackRF device found" (as today)

**Observer polling task** (replaces hardware task in observer mode):
```rust
tokio::spawn(async move {
    let mut last_owner_cpu_ticks: Option<(u64, std::time::Instant)> = None;
    loop {
        if let Some(info) = sysfs::find_hackrf() {
            let owner = sysfs::find_owner(info.bus, info.dev);
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.observer_device   = Some(format!("{} · {}", info.product, info.manufacturer));
            m.observer_serial   = Some(info.serial);
            m.observer_usb      = Some(format!("High Speed ({} Mbit/s) · {} mA · Bus {}, Port {}",
                                    info.speed_mbits, info.max_power_ma, info.bus, info.dev));
            m.observer_connected = info.connected_secs.map(fmt_duration);
            if let Some(o) = owner {
                m.observer_owner        = Some(format!("{} (PID {})", o.name, o.pid));
                m.observer_cmdline      = Some(o.cmdline);
                m.observer_owner_cpu    = o.cpu_pct;
                m.observer_owner_ram_mb = o.rss_mb;
                m.observer_owner_uptime = Some(fmt_duration(o.running_secs));
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
});
```

**Key handler gating:**  
Space, LNA/VGA/AMP, frequency, sample rate — all check `if self.device.is_some()` before acting. Silent no-op in observer mode (footer explains why).

**Footer in observer mode:**
```
Observer Mode — [Q] Quit  [?] Help
```

### New: `src/ui/observer.rs` — `ObserverPanel`

Renders all `observer_*` fields from state. Uses `Color::Yellow` for the mode banner,
normal colors for device/owner info. Gracefully handles `None` fields with "—".

### New preset: `"observer"`

```
Top:    header (3 lines)
Body:   observer panel (Left, 100%)
Body:   system_resources (Right, 40%)
Bottom: log (5 lines)
Bottom: footer (3 lines)
```

Set automatically when entering observer mode. User can still switch presets manually.

---

## Behaviour summary

| Scenario | Behaviour |
|---|---|
| No HackRF connected | Bail with "No HackRF device found" (as today) |
| HackRF free | Normal mode (no change) |
| HackRF in use, same user | Observer mode, full owner info |
| HackRF in use, different user | Observer mode, device info only, owner shown as "unknown process" |
| Device freed while sdrtop is running | Not handled — user restarts sdrtop |

---

## Out of scope

- Auto-recovery when device becomes free (future improvement)
- USB transfer throughput (needs usbmon + root)
- Current frequency/gain of the owner app (not accessible)

---

## Files touched

| File | Change |
|---|---|
| `src/hardware/sysfs.rs` | New — sysfs + /proc scanning |
| `src/hardware/mod.rs` | Export sysfs module |
| `src/state.rs` | Observer fields |
| `src/app.rs` | Optional device, fallback logic, observer polling task, key gating |
| `src/ui/observer.rs` | New panel |
| `src/ui/mod.rs` | Export ObserverPanel |
| `src/config.rs` | Add "observer" preset |
