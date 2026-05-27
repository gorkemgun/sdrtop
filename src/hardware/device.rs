use libc::{c_int, c_void};
use std::ffi::CStr;
use std::sync::Mutex;

use crate::state::SdrMetrics;
use super::ffi::*;

pub struct Device(*mut c_void);

// Safety: libhackrf is thread-safe for status polling and streaming control
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

pub extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int {
    unsafe {
        let t = &*transfer;
        let metrics_ptr = t.rx_ctx as *const Mutex<SdrMetrics>;
        if !metrics_ptr.is_null() {
            if let Ok(mut m) = (*metrics_ptr).lock() {
                m.bytes_since_last_poll += t.valid_length as u64;
            }
        }
    }
    0
}

#[allow(dead_code)]
impl Device {
    pub fn open() -> anyhow::Result<Self> {
        unsafe {
            let init_res = hackrf_init();
            if init_res != 0 {
                let err = CStr::from_ptr(hackrf_error_name(init_res)).to_string_lossy();
                anyhow::bail!("Failed to initialize libhackrf: {}", err);
            }

            let list_ptr = hackrf_device_list();
            if list_ptr.is_null() {
                hackrf_exit();
                anyhow::bail!("Failed to retrieve HackRF device list.");
            }

            let list = &*list_ptr;
            let count = list.devicecount as usize;

            if count == 0 {
                hackrf_device_list_free(list_ptr);
                hackrf_exit();
                anyhow::bail!(
                    "No HackRF device found. Please connect your device and try again."
                );
            }

            let selected_index = if count == 1 {
                0
            } else {
                println!("Multiple HackRF devices found:");
                let mut valid_count = 0;
                if !list.serial_numbers.is_null() {
                    for i in 0..count {
                        let serial_ptr = *list.serial_numbers.add(i);
                        if !serial_ptr.is_null() {
                            let serial = CStr::from_ptr(serial_ptr).to_string_lossy();
                            println!("[{}] Serial: {}", i, serial);
                            valid_count += 1;
                        }
                    }
                }

                if valid_count == 0 {
                    hackrf_device_list_free(list_ptr);
                    hackrf_exit();
                    anyhow::bail!("No valid serial numbers found for connected devices.");
                }
                print!("Select device index [0-{}]: ", count - 1);
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let idx = input.trim().parse::<usize>().unwrap_or(usize::MAX);
                if idx >= count {
                    hackrf_device_list_free(list_ptr);
                    hackrf_exit();
                    anyhow::bail!("Invalid device index selected.");
                }
                idx
            };

            let mut ptr = std::ptr::null_mut();
            let res = hackrf_device_list_open(list_ptr, selected_index as c_int, &mut ptr);
            hackrf_device_list_free(list_ptr);

            if res != 0 {
                let err = CStr::from_ptr(hackrf_error_name(res)).to_string_lossy();
                hackrf_exit();
                anyhow::bail!("Failed to open HackRF device: {} (code {})", err, res);
            }

            Ok(Device(ptr))
        }
    }

    pub fn version(&self) -> anyhow::Result<String> {
        let mut buf = [0i8; 64];
        unsafe {
            if hackrf_version_string_read(self.0, buf.as_mut_ptr(), 63) != 0 {
                anyhow::bail!("Failed to read firmware version");
            }
            Ok(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
        }
    }

    pub fn is_streaming(&self) -> bool {
        unsafe { hackrf_is_streaming(self.0) == 1 }
    }

    pub fn start_rx(
        &self,
        callback: HackrfTransferCallback,
        user_param: *mut c_void,
    ) -> anyhow::Result<()> {
        unsafe {
            if hackrf_start_rx(self.0, callback, user_param) != 0 {
                anyhow::bail!("Failed to start RX streaming");
            }
        }
        Ok(())
    }

    pub fn stop_rx(&self) -> anyhow::Result<()> {
        unsafe {
            if hackrf_stop_rx(self.0) != 0 {
                anyhow::bail!("Failed to stop RX streaming");
            }
        }
        Ok(())
    }

    pub fn set_lna_gain(&self, gain: u32) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_lna_gain(self.0, gain) != 0 {
                anyhow::bail!("Failed to set LNA gain");
            }
        }
        Ok(())
    }

    pub fn set_vga_gain(&self, gain: u32) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_vga_gain(self.0, gain) != 0 {
                anyhow::bail!("Failed to set VGA gain");
            }
        }
        Ok(())
    }

    pub fn set_sample_rate(&self, sample_rate: f64) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_sample_rate(self.0, sample_rate) != 0 {
                anyhow::bail!("Failed to set sample rate");
            }
        }
        Ok(())
    }

    pub fn set_frequency(&self, freq_hz: u64) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_freq(self.0, freq_hz) != 0 {
                anyhow::bail!("Failed to set frequency");
            }
        }
        Ok(())
    }

    pub fn set_amp_enable(&self, enable: bool) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_amp_enable(self.0, enable as u8) != 0 {
                anyhow::bail!("Failed to set AMP enable");
            }
        }
        Ok(())
    }

    pub fn board_id(&self) -> anyhow::Result<u8> {
        let mut id = 0u8;
        unsafe {
            if hackrf_board_id_read(self.0, &mut id) != 0 {
                anyhow::bail!("Failed to read board ID");
            }
            Ok(id)
        }
    }

    pub fn board_name(&self, id: u8) -> String {
        unsafe {
            let ptr = hackrf_board_id_name(id);
            if ptr.is_null() {
                return "Unknown".to_string();
            }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    pub fn serial_number(&self) -> anyhow::Result<String> {
        let mut data = ReadPartidSerialno {
            part_id: [0; 2],
            serial_no: [0; 4],
        };
        unsafe {
            if hackrf_board_partid_serialno_read(self.0, &mut data) != 0 {
                anyhow::bail!("Failed to read serial number");
            }
            let s = data.serial_no;
            Ok(format!(
                "{:08x}{:08x}{:08x}{:08x}",
                s[0], s[1], s[2], s[3]
            ))
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if hackrf_is_streaming(self.0) == 1 {
                let _ = hackrf_stop_rx(self.0);
            }
            hackrf_close(self.0);
            hackrf_exit();
        }
    }
}
