use std::mem::size_of;
use std::path::Path;

use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, HMODULE};
use windows::Win32::System::Diagnostics::Debug::{
    SymGetModuleInfo64, SymInitialize, SymLoadModuleEx, SymSetOptions, IMAGEHLP_MODULE64,
    SYMOPT_UNDNAME,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleExA;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

use crate::constants::*;

pub unsafe fn get_guid() -> String {
    let modinfo = get_shell32_modinfo();
    let sig = modinfo.PdbSig70.to_u128();
    let age = modinfo.PdbAge;
    // format as hex as michael expects
    format!("{sig:032X}{age:X}")
}

pub struct ExplorerHandle(HANDLE);

impl ExplorerHandle {
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for ExplorerHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub unsafe fn get_shell32_offset() -> u64 {
    let modinfo = get_shell32_modinfo();
    modinfo.BaseOfImage
}

pub unsafe fn get_explorer_handle() -> ExplorerHandle {
    let explorerid =
        // initialize sysinfo with process info
        sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::nothing()
                .with_processes(sysinfo::ProcessRefreshKind::everything()),
        )
            // get explorer
            .processes()
            .values()
            .find(|proc| {
                if let Some(p) = proc.exe() {
                    p == Path::new(r"C:\Windows\explorer.exe")
                } else {
                    false
                }
            })
            .expect("explorer.exe is not running")
            // get PID
            .pid()
            .as_u32();

    ExplorerHandle(
        OpenProcess(PROCESS_ALL_ACCESS, false, explorerid)
            .expect("failed to open explorer.exe with full access"),
    )
}

pub unsafe fn get_shell32_modinfo() -> IMAGEHLP_MODULE64 {
    // get info of shell32.dll using running explorer.exe

    let explorerhandle = get_explorer_handle();

    // let currentprocess = GetCurrentProcess();
    SymInitialize(explorerhandle.raw(), PCSTR::null(), true)
        .expect("failed to initialize the Windows symbol handler");
    SymSetOptions(SYMOPT_UNDNAME);
    let nullterminatedpath = format!("{}\0", SHELL32_PATH);
    // dbg!(&nullterminatedpath);
    let name = PCSTR::from_raw(nullterminatedpath.as_ptr());
    let mut module = HMODULE::default();
    GetModuleHandleExA(0, name, &mut module as *mut HMODULE).expect("failed to locate shell32.dll");
    // let module = LoadLibraryExA(name, HANDLE::default(), LOAD_LIBRARY_FLAGS::default()).unwrap();
    let r = SymLoadModuleEx(
        explorerhandle.raw(), // target process
        None,                 // handle to image - not used
        name,                 // name of image file
        PCSTR::null(),        // name of module - not required
        module.0 as u64,      // base address - not required
        0,                    // size of image - not required
        None,
        None,
    );
    assert_ne!(
        r,
        0,
        "failed to load shell32.dll symbols: {:?}",
        GetLastError()
    );
    let mut modinfo = IMAGEHLP_MODULE64 {
        SizeOfStruct: size_of::<IMAGEHLP_MODULE64>() as u32,
        ..Default::default()
    };
    SymGetModuleInfo64(
        explorerhandle.raw(),
        module.0 as u64,
        &mut modinfo as *mut IMAGEHLP_MODULE64,
    )
    .expect("failed to read shell32.dll symbol information");
    // dbg!(modinfo);
    modinfo
}
