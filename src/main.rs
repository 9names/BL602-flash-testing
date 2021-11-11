#![no_std]
#![no_main]

use bl602_hal as hal;
use core::fmt::Write;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
};
use embedded_hal::delay::blocking::DelayMs;
use panic_rtt_target as _;

use bl602_rom_wrapper::rom::{self, sflash as sflash, xip_sflash as xip};
mod flash;
mod xip_flash;
use rtt_target::{rtt_init_print, rprintln};

#[riscv_rt::entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Program start");
    let dp = pac::Peripherals::take().unwrap();
    let mut parts = dp.GLB.split();

    // Set up all the clocks we need
    // Minimal clock setup here - PLL was not working correctly, probably don't want it anyway
    let clocks = Strict::new()
        .freeze(&mut parts.clk_cfg);

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    rprintln!("Ready to test flash routines");

    // Disable the flash cache, get rid of the flash offset, and disconnect the flash from the flash accelerator
    flash::Init(1,2,3);

    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    // JEDEC ID is 3 bytes, make sure writebuf is at least that big
    // I've tested it, it does only write 3 bytes :D
    let mut writebuf:[u8;3] = [0;3];
    let _ = sflash::SFlash_GetJedecId(&mut cfg, writebuf.as_mut_ptr());

    rprintln!("JEDEC id after init");
	for c in writebuf {
        rprintln!("{:x}", c);
    }

    flash::UnInit(1);
    rprintln!("JEDEC id after uninit:");
    let _ = sflash::SFlash_GetJedecId(&mut cfg, writebuf.as_mut_ptr());
    for c in writebuf {
        rprintln!("{:x}", c);
    }

    rprintln!("Testing done!");
    //panic!("ded");
    loop {
        // Could do a blink here if you want better feedback.
        // I'm using the bl602 EVB, so the LEDs are already busy being JTAG
        d.delay_ms(1000).unwrap();
    }
}
