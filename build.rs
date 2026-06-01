fn main() {
    // Requires libhackrf >= 2023.01.1 (hackrf_board_rev_read,
    // hackrf_usb_api_version_read). Many .pc files omit the version field so
    // atleast_version() fails even on a correct install — just probe and let
    // the linker error on missing symbols if the library is too old.
    //
    // Install: apt install libhackrf-dev  (Bookworm / Ubuntu 24.04+)
    // Older distros: build from source — https://github.com/greatscottgadgets/hackrf
    if let Err(e) = pkg_config::probe_library("libhackrf") {
        panic!(
            "libhackrf not found ({}). \
             Install: apt install libhackrf-dev  \
             (requires Raspberry Pi OS Bookworm or Ubuntu 24.04+)",
            e
        );
    }
}