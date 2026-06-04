# Supported Hardware

← [Back](README.md)

---

## What works today

| Device | Status |
|--------|--------|
| HackRF One | Fully supported — spectrum, waterfall, all diagnostics |
| RTL-SDR (R820T / R828D / E4000) | 🧪 **Experimental — just landed.** Full spectrum / waterfall / lab stack, single tuner gain + AGC. Community-contributed; **needs wider testing & feedback** |
| PortaPack H4M (Mayhem) | In development — telemetry panel via USB serial |

sdrtop is built and tested on real hardware. Support is only added after physical testing — no guessing from documentation alone. Datasheets have been known to fib; an oscilloscope rarely does.

> **A word on the RTL-SDR backend:** it was community-contributed and verified on the contributor's hardware (FM reception, tuner gain, AGC, sweep). I don't own an RTL-SDR yet, so I can't test it here — and there's a whole zoo of clones with different tuners and quirks. If you run sdrtop on one, please [open an issue](../../../issues) with what worked and what didn't; that's what turns 🧪 into ✅.

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

The **RTL-SDR backend is now in** (community-contributed) — by far the most common SDR dongle, and the biggest single jump in who can use sdrtop. It stays experimental until I can get a dongle and verify it myself, so your testing and feedback carry it the rest of the way. After that come Airspy and the wider SoapySDR ecosystem.

If you'd like to support this, contributions go directly toward hardware purchases:

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/mustang6139)
