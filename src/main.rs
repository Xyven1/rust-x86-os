#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::arch::x86_64::_rdtsc;
use core::panic::PanicInfo;
use raw_cpuid::CpuId;
pub mod terminal;

entry_point!(kernel_main);

/* impl Kernel {
    fn new(boot_info: &'static BootInfo) -> Kernel {
        Kernel {
            boot_info,
        }
    }
} */

/* struct kernel {
    boot_info: &'static BootInfo,
} */
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // turn the screen gray
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        for (i, byte) in framebuffer.buffer_mut().iter_mut().enumerate() {
            *byte = (i % 255) as u8;
        }
        let info = framebuffer.info().clone();
        terminal::WRITER
            .get_or_init(move || terminal::LockedWriter::new(framebuffer.buffer_mut(), info));
    }
    let start_time = get_time();
    printf!("Test\n");
    let mut last_print = get_time();
    loop {
        let time = get_time();
        if time - last_print > 1000000000 {
            printf!("Time: {}s\n", (time - start_time) / 1000000000);
            last_print = time;
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

fn get_time() -> u64 {
    let mut time = 0;
    // Check if the CPU supports `RDTSC`.
    let cpu_id = CpuId::new();
    if let Some(feature_info) = cpu_id.get_feature_info() {
        if feature_info.has_tsc() {
            let value = unsafe {
                // SAFETY: We checked that the cpu supports `RDTSC` and we run in ring 0.
                core::arch::x86_64::_rdtsc()
            };
            time = value;
        }
    }
    time
}
