fn main() {
    // Use pkg-config to find libhackrf and provide the correct linker flags.
    // This is more robust than hardcoding #[link] attributes in source code.
    if let Err(e) = pkg_config::probe_library("libhackrf") {
        panic!("Failed to find libhackrf via pkg-config: {}. Ensure libhackrf-dev is installed.", e);
    }
}