// These are for verify, remove them if we don't implement that
use core::{intrinsics::transmute, ops::Range, slice};

use bl602_rom_wrapper::rom::xip_sflash::XIP_SFlash_Read_Need_Lock;
use bl602_rom_wrapper::rom::{SPI_Flash_Cfg_Type, sflash as sflash, xip_sflash as xip};

use bl602_rom_wrapper::rom::{
    self,
    sf_ctrl::{SF_Ctrl_Set_Flash_Image_Offset,SF_Ctrl_Set_Owner},
    SF_Ctrl_Mode_Type_SF_CTRL_QPI_MODE, SF_Ctrl_Owner_Type_SF_CTRL_OWNER_IAHB,
    SF_Ctrl_Owner_Type_SF_CTRL_OWNER_SAHB,
};

const BASE_ADDRESS: u32 = 0x2300_0000;

/// Erase the sector at the given address in flash
///
/// `Return` - 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub extern "C" fn EraseSector(adr: u32) -> i32 {
    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    match xip::XIP_SFlash_Erase_With_Lock(&mut cfg, adr, 4096) {
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
    // 0
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
    match xip::XIP_SFlash_Write_With_Lock(&mut cfg, adr, buf, sz) {
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
pub unsafe extern "C" fn Verify(adr: u32, sz: u32, buf: *mut u8, expected: *mut i16, found: *mut i16) -> u32 {
    let mut readbuf: [u8; 4096] = [0; 4096];
    let verifybuf = slice::from_raw_parts(buf, sz as usize);

    if sz > 4096 {
        return 0;
    }    

    xip::XIP_SFlash_Read_Via_Cache_Need_Lock(adr, readbuf.as_mut_ptr(), sz);

    for i in 0..sz as usize {
        if verifybuf[i] != readbuf[i] {
            *expected = verifybuf[i] as i16;
            *found = readbuf[i] as i16;
            return adr + i as u32;
        }
    }
    adr + sz
}

/// Not a CMSIS function. For testing
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn Read(adr: u32, sz: u32, readbuf: *mut u8) -> u32 {
    let mut cfg = rom::flashconfig::winbond_80_ew_cfg();
    let readbuf = slice::from_raw_parts_mut(readbuf, sz as usize);

    
    if xip::XIP_SFlash_Read_Need_Lock(&mut cfg, adr, readbuf.as_mut_ptr(), sz) != 0 {
        return 0;
    }

    return 2;
}
