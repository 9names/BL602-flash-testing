#![no_std]
#![no_main]

use bl602_hal as hal;
use core::fmt::Write;
use embedded_hal::delay::blocking::DelayMs;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
};
use panic_rtt_target as _;

use bl602_rom_wrapper::rom::{
    self,
    sflash::{self, SFlash_Chip_Erase},
    xip_sflash as xip, SF_Ctrl_Mode_Type_SF_CTRL_QPI_MODE,
};
mod flash;
mod xip_flash;
use rtt_target::{rprint, rprintln, rtt_init_print};

// Could calculate this on the fly, I'm lazy
static mut TEST_BUFFER: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173,
    174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
    231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255,
];

// All zero test buffer
// static mut ZERO_BUFFER: [u8; 256] = [0;256];
// All one test buffer
// static mut TEST_BUFFER: [u8; 256] = [0xff;256];

#[riscv_rt::entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Program start");
    let dp = pac::Peripherals::take().unwrap();
    let mut parts = dp.GLB.split();

    // Set up all the clocks we need
    // Minimal clock setup here - PLL was not working correctly, probably don't want it anyway
    let clocks = Strict::new().freeze(&mut parts.clk_cfg);

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    rprintln!("Ready to test flash routines");

    // Disable the flash cache, get rid of the flash offset, and disconnect the flash from the flash accelerator
    flash::Init(1, 2, 3);

    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    // JEDEC ID is 3 bytes, make sure writebuf is at least that big
    // I've tested it, it does only write 3 bytes :D
    let mut jedec_buf: [u8; 3] = [0; 3];
    // Source code for this function is at
    // https://github.com/bouffalolab/bl_iot_sdk/blob/07ceb89192cd720e1645e6c37081c85960a33580/components/platform/soc/bl602/bl602_std/bl602_std/StdDriver/Src/bl602_sflash.c#L717
    let _ = sflash::SFlash_GetJedecId(&mut cfg, jedec_buf.as_mut_ptr());
    rprintln!(
        "JEDEC id after init: {:x} {:x} {:x}",
        jedec_buf[0],
        jedec_buf[1],
        jedec_buf[2]
    );

    // The sflash functions expect addresses starting at 0 for flash.
    // 0 == 2300_0000 if flash offset 0, or 2301_0000 if using the default application offset

    // SFlash_Erase internally handles erasing of different block sizes
    // If erase size >= 64KB, it will call SFlash_Blk64_Erase until < 64KB
    // If erase size >= 32KB, it will call SFlash_Blk32_Erase until < 32KB
    // If erase size < 32KB, it will call SFlash_Sector_Erase which will erase
    // flashCfg->sectorSize * 1024 bytes until it reaches the end.
    // In our case, sectorSize = 4, so 4KB is the smallest erase size
    // Source code for this function is at
    // https://github.com/bouffalolab/bl_iot_sdk/blob/07ceb89192cd720e1645e6c37081c85960a33580/components/platform/soc/bl602/bl602_std/bl602_std/StdDriver/Src/bl602_sflash.c#L545
    const SIZE_OF_FLASH: u32 = 2097152;
    const WRITE_SIZE: usize = 256;
    const LINE_LENGTH: u32 = 128;
    //sflash::SFlash_Erase(&mut cfg, 0, 256);
    rprintln!("\nPerforming chip erase:");
    SFlash_Chip_Erase(&mut cfg);
    rprintln!("Done with erase");
    let writelen = unsafe { TEST_BUFFER.len() } as u32;
    // Source code for this function is at
    // https://github.com/bouffalolab/bl_iot_sdk/blob/07ceb89192cd720e1645e6c37081c85960a33580/components/platform/soc/bl602/bl602_std/bl602_std/StdDriver/Src/bl602_sflash.c#L594
    rprintln!("\nWriting to flash starting at flash address 0 (mapped offset 0x2300_0000)");
    let mut newline_counter: u32 = 0;
    for adr in (0..SIZE_OF_FLASH).step_by(WRITE_SIZE) {
        rprint!(".");
        sflash::SFlash_Program(
            &mut cfg,
            SF_Ctrl_Mode_Type_SF_CTRL_QPI_MODE,
            adr,
            unsafe { TEST_BUFFER.as_mut_ptr() },
            writelen,
        );
        newline_counter += 1;
        if newline_counter % LINE_LENGTH == LINE_LENGTH - 1 {
            rprintln!("");
        }
    }

    flash::UnInit(1);

    rprintln!("\n\nDone writing.\nData at flash offset 0x2300_0000 (read directly)");
    newline_counter = 0;
    for adr in (0..SIZE_OF_FLASH).step_by(WRITE_SIZE) {
        rprint!(".");
        for i in 0..WRITE_SIZE as u32 {
            let addr_mapped = 0x2300_0000 + i + adr;
            let data_ptr = addr_mapped as *const u8;
            unsafe {
                let data = data_ptr.read_volatile();
                let expected = TEST_BUFFER[i as usize];
                if data != expected {
                    rprintln!(
                        "\nAddr {:08x} fail, expected {:02x} found {:02x}",
                        addr_mapped,
                        expected,
                        data
                    );
                }
            }
        }
        if newline_counter % LINE_LENGTH == LINE_LENGTH - 1 {
            rprintln!("");
        }
    }

    rprintln!("\nTesting done!");
    loop {
        // Could do a blink here if you want better feedback.
        // I'm using the bl602 EVB, so the LEDs are already busy being JTAG
        d.delay_ms(1000).unwrap();
    }
}
