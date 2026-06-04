# Supported Hardware

← [Back](README.md)

---

## What works today

| Device | Status |
|--------|--------|
| HackRF One | Fully supported — spectrum, waterfall, all diagnostics |
| RTL-SDR (R820T / R828D / E4000) | ✅ **Working *(new)*.** Full spectrum / waterfall / lab stack, single tuner gain + AGC. Community-contributed, confirmed on hardware; **test on your clone & report** |
| PortaPack H4M (Mayhem) | In development — telemetry panel via USB serial |

sdrtop is built and tested on real hardware. Support is only added after physical testing — no guessing from documentation alone. Datasheets have been known to fib; an oscilloscope rarely does.

> **A word on the RTL-SDR backend:** it works — community-contributed and confirmed on real hardware, including normal RX and observer mode (FM reception, tuner gain, AGC, sweep). The only catch is the zoo of RTL clones with different tuners and quirks, and I don't own one to test them all. If you run sdrtop on yours, please [open an issue](../../../issues) with what worked and what didn't — that's how it gets from "works on the units we've tried" to "works, full stop."

---

## Host platforms

| Platform | Status |
|----------|--------|
| x86-64 Linux | Fully supported |
| Raspberry Pi (Pi 2 and newer, 64-bit Raspberry Pi OS Bookworm) | Supported — lower sample rates on older Pis |
| ARM / Android (Termux) | Builds and runs; needs a root-capable USB stack to reach the device |

sdrtop needs **libhackrf 2023.01.1 or newer** (the version in Raspberry Pi OS Bookworm and Ubuntu 24.04). Older distributions need libhackrf built from source. For the RTL-SDR backend it also links **librtlsdr** (`librtlsdr-dev` on Debian/Ubuntu, `rtl-sdr` on Arch).

---

## What's coming

| Device | Status | Notes |
|--------|--------|-------|
| Airspy Mini | Planned | Needs hardware to test |
| Airspy HF+ Discovery | Planned | Needs hardware to test |
| LimeSDR / bladeRF / SDRplay / PlutoSDR | Planned | Wide range of devices, needs hardware |

---

## Supporting hardware development

New device support requires physically owning and testing the hardware. Development here runs on a HackRF One and a PortaPack H4M.

The **RTL-SDR backend is now in and working** (community-contributed, confirmed on real hardware) — by far the most common SDR dongle, and the biggest single jump in who can use sdrtop. I'll pick up a dongle to test it here too, but with the variety of RTL clones out there, your testing and feedback are what carry it the rest of the way. After that come Airspy and the wider SoapySDR ecosystem.

If you'd like to support this, contributions go directly toward hardware purchases:

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/mustang6139)
