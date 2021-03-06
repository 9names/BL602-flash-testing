// These are for verify, remove them if we don't implement that
use core::slice;

use bl602_rom_wrapper::rom::{sflash, xip_sflash as xip};

use bl602_rom_wrapper::rom::{
    self,
    sf_ctrl::{SF_Ctrl_Set_Flash_Image_Offset, SF_Ctrl_Set_Owner},
    SF_Ctrl_Mode_Type_SF_CTRL_QPI_MODE, SF_Ctrl_Owner_Type_SF_CTRL_OWNER_IAHB,
    SF_Ctrl_Owner_Type_SF_CTRL_OWNER_SAHB,
};

const BASE_ADDRESS: u32 = 0x2300_0000;

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

/// Erase the sector at the given address in flash
///
/// `Return` - 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub extern "C" fn EraseSector(adr: u32) -> i32 {
    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    let addr_native = adr.wrapping_sub(BASE_ADDRESS);
    let target_sector = addr_native >> 12;
    match sflash::SFlash_Sector_Erase(&mut cfg, target_sector) {
        0 => 0,
        _ => 1,
    }
}

/// Erase the chip
///
/// `Return` - 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub extern "C" fn EraseChip() -> i32 {
    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    match sflash::SFlash_Chip_Erase(&mut cfg) {
        0 => 0,
        _ => 1,
    }
}

/// Initializes the microcontroller for Flash programming. Returns 0 on Success, 1 otherwise
///
/// This is invoked whenever an attempt is made to download the program to Flash.
///
///  # Arguments
///
/// `adr` - specifies the base address of the device.
///
/// `clk` - specifies the clock frequency for prgramming the device.
///
/// `fnc` - is a number: 1=Erase, 2=Program, 3=Verify, to perform different init based on command
#[no_mangle]
#[inline(never)]
pub extern "C" fn Init(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    sflash::SFlash_Cache_Read_Disable();
    SF_Ctrl_Set_Flash_Image_Offset(0);
    SF_Ctrl_Set_Owner(SF_Ctrl_Owner_Type_SF_CTRL_OWNER_SAHB);
    0
}

/// Write code into the Flash memory. Call this to download a program to Flash. Returns 0 on Success, 1 otherwise
///
/// As Flash memory is typically organized in blocks or pages, parameters must not cross alignment boundaries of flash pages.
/// The page size is specified in the struct FlashDevice with the value Program Page Size.
/// # Arguments
///
/// `adr` - specifies the start address of the page that is to be programmed. It is aligned by the host programming system to a start address of a flash page
///
/// `sz` -  specifies the data size in the data buffer. The host programming system ensures that page boundaries are not crossed
///
/// `buf` - points to the data buffer containing the data to be programmed
#[no_mangle]
#[inline(never)]
pub extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *mut u8) -> i32 {
    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    let addr = adr.wrapping_sub(BASE_ADDRESS);
    match sflash::SFlash_Program(&mut cfg, SF_Ctrl_Mode_Type_SF_CTRL_QPI_MODE, addr, buf, sz) {
        0 => 0,
        _ => 1,
    }
}

/// De-initializes the microcontroller after Flash programming. Returns 0 on Success, 1 otherwise
///
/// This is invoked at the end of an erasing, programming, or verifying step.
///
///  # Arguments
///
/// `fnc` - is a number: 1=Erase, 2=Program, 3=Verify, to perform different de-init based on command
#[no_mangle]
#[inline(never)]
pub extern "C" fn UnInit(_fnc: u32) -> i32 {
    // Put the flash controller back into memory-mapped mode
    // TODO: re-enable cache
    SF_Ctrl_Set_Owner(SF_Ctrl_Owner_Type_SF_CTRL_OWNER_IAHB);
    // TODO: work out where to set this to, whether we can after verify, etc
    //SF_Ctrl_Set_Flash_Image_Offset(0x11000);
    sflash::SFlash_Cache_Flush();
    0
}

/// Compares the content of the Flash memory with the program code *buf.
/// Returns (adr+sz) on success, failing address otherwise
///
/// This is invoked at the end of an erasing, programming, or verifying step.
///
/// # Arguments
///
/// `adr` - specifies the start address of the page that is to be verified.
///
/// `sz` -  specifies the data size in the data buffer
///
/// `buf` - data to be compared
/// # Safety
/// We're calling into C data structures, there's no safety here
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn Verify(
    adr: u32,
    sz: u32,
    buf: *mut u8,
    expected: *mut i16,
    found: *mut i16,
) -> u32 {
    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    let addr = adr.wrapping_sub(BASE_ADDRESS);
    let mut readbuf: [u8; 4096] = [0; 4096];
    let verifybuf = slice::from_raw_parts(buf, sz as usize);

    if sz > 4096 {
        return 0;
    }

    if sflash::SFlash_Read(
        &mut cfg,
        SF_Ctrl_Mode_Type_SF_CTRL_QPI_MODE,
        0,
        addr,
        readbuf.as_mut_ptr(),
        sz,
    ) != 0
    {
        return 1;
    }

    for i in 0..sz as usize {
        if verifybuf[i] != readbuf[i] {
            *expected = verifybuf[i] as i16;
            *found = readbuf[i] as i16;
            return adr + i as u32;
        }
    }
    adr + sz
}

const fn sectors() -> [FlashSector; 512] {
    let mut sectors = [FlashSector::default(); 512];

    // 4K sectors starting at address 0
    sectors[0] = FlashSector {
        size: 0x1000,
        address: 0x0,
    };
    sectors[1] = SECTOR_END;

    sectors
}

#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
#[link_section = "DeviceData"]
pub static FlashDevice: FlashDeviceDescription = FlashDeviceDescription {
    vers: 0x0101,
    dev_name: [0u8; 128],
    dev_type: 5,
    dev_addr: BASE_ADDRESS,
    device_size: 0x1e8480,
    page_size: 256,
    _reserved: 0,
    empty: 0xff,
    program_time_out: 5,
    erase_time_out: 20000,
    flash_sectors: sectors(),
};

#[repr(C)]
pub struct FlashDeviceDescription {
    vers: u16,
    dev_name: [u8; 128],
    dev_type: u16,
    dev_addr: u32,
    device_size: u32,
    page_size: u32,
    _reserved: u32,
    empty: u8,
    program_time_out: u32,
    erase_time_out: u32,

    flash_sectors: [FlashSector; 512],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FlashSector {
    size: u32,
    address: u32,
}

impl FlashSector {
    const fn default() -> Self {
        FlashSector {
            size: 0,
            address: 0,
        }
    }
}

const SECTOR_END: FlashSector = FlashSector {
    size: 0xffff_ffff,
    address: 0xffff_ffff,
};
