//! A program's own microphone or speaker (e.g. claude.exe's dictation microphone) without
//! changing the system defaults — what Settings > Sound > App volume and device preferences
//! does, through the undocumented IAudioPolicyConfigFactory (as EarTrumpet does). Windows
//! remembers the choice for the program's path, so one running process is enough.

use crate::error::{AppError, Result};

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    use crate::error::{AppError, Result};

    const MMDEV: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio";
    const PKEY_NAME: &str = "{a45c254e-df1c-4efd-8020-67d146a850e0},2";
    const PKEY_IFACE: &str = "{b3f8fa53-0004-438e-9003-51a46e139bfc},6";

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);

    #[link(name = "runtimeobject")]
    extern "system" {
        fn RoInitialize(kind: i32) -> i32;
        fn WindowsCreateString(src: *const u16, len: u32, out: *mut *mut c_void) -> i32;
        fn RoGetActivationFactory(class: *mut c_void, iid: *const Guid, out: *mut *mut c_void) -> i32;
    }

    /// [(endpoint id, description)] of active devices ("input" = capture, else render).
    pub fn devices(kind: &str) -> Vec<(String, String)> {
        let base = format!(r"{MMDEV}\{}", if kind == "input" { "Capture" } else { "Render" });
        let Ok(root) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(&base) else { return Vec::new() };
        let prefix = if kind == "input" { "{0.0.1.00000000}." } else { "{0.0.0.00000000}." };
        root.enum_keys()
            .flatten()
            .filter_map(|sub| {
                let key = root.open_subkey(&sub).ok()?;
                (key.get_value::<u32, _>("DeviceState").ok()? == 1).then_some(())?;
                let props = key.open_subkey("Properties").ok()?;
                let name: String = props.get_value(PKEY_NAME).ok()?;
                let iface: String = props.get_value(PKEY_IFACE).ok()?;
                Some((format!("{prefix}{sub}"), format!("{name} ({iface})")))
            })
            .collect()
    }

    fn hstring(s: &str) -> *mut c_void {
        let wide: Vec<u16> = s.encode_utf16().collect();
        let mut h = std::ptr::null_mut();
        // SAFETY: the buffer is copied into the HSTRING.
        unsafe { WindowsCreateString(wide.as_ptr(), wide.len() as u32, &mut h) };
        h
    }

    pub fn set(pid: u32, kind: &str, device_id: Option<&str>) -> Result<()> {
        let build = windows_version::OsVersion::current().build;
        let iid = if build >= 21390 {
            Guid(0xab3d4648, 0xe242, 0x459f, [0xb0, 0x2f, 0x54, 0x1c, 0x70, 0x30, 0x63, 0x24])
        } else {
            Guid(0x2a59116d, 0x6c4f, 0x45e0, [0xa7, 0x4f, 0x70, 0x7e, 0x3f, 0xef, 0x92, 0x58])
        };
        let iface = if kind == "input" { "#{2eef81be-33fa-4800-9670-1cd474972c3f}" } else { "#{e6327cad-dcec-4949-ae8a-991e976a79d2}" };
        let flow = i32::from(kind == "input");
        // SAFETY: WinRT activation of a system class; the vtable layout (IInspectable + 19 methods,
        // SetPersistedDefaultAudioEndpoint at 25) is the one EarTrumpet uses on these builds.
        unsafe {
            RoInitialize(1);
            let mut factory: *mut c_void = std::ptr::null_mut();
            let hr = RoGetActivationFactory(hstring("Windows.Media.Internal.AudioPolicyConfig"), &iid, &mut factory);
            if hr != 0 || factory.is_null() {
                return Err(AppError::Audio(format!("RoGetActivationFactory: 0x{:08x}", hr as u32)));
            }
            let vtable = *(factory as *const *const *const c_void);
            type SetFn = unsafe extern "system" fn(*mut c_void, u32, i32, i32, *mut c_void) -> i32;
            let set: SetFn = std::mem::transmute(*vtable.add(25));
            let h = device_id.map(|d| hstring(&format!(r"\\?\SWD#MMDEVAPI#{d}{iface}"))).unwrap_or(std::ptr::null_mut());
            for role in [0, 1] {
                let hr = set(factory, pid, flow, role, h);
                if hr != 0 {
                    return Err(AppError::Audio(format!("SetPersistedDefaultAudioEndpoint: 0x{:08x}", hr as u32)));
                }
            }
        }
        Ok(())
    }
}

/// Gives the process `pid` the device whose description contains `wanted` ("default" resets).
pub fn assign(pid: u32, kind: &str, wanted: &str) -> Result<String> {
    #[cfg(windows)]
    {
        if wanted == "default" {
            imp::set(pid, kind, None)?;
            return Ok("Przywrocono urzadzenie domyslne.".into());
        }
        let w = wanted.to_lowercase();
        let (id, desc) = imp::devices(kind).into_iter().find(|(_, d)| d.to_lowercase().contains(&w)).ok_or_else(|| AppError::msg(format!("Nie znaleziono urzadzenia '{wanted}'.")))?;
        imp::set(pid, kind, Some(&id))?;
        Ok(format!("Ustawiono: {desc}"))
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, kind, wanted);
        Err(AppError::Audio("per-app audio needs Windows".into()))
    }
}

/// "[input]\n  ...\n[output]\n  ..." of the active devices.
pub fn list() -> String {
    #[cfg(windows)]
    {
        ["input", "output"].iter().map(|k| format!("[{k}]\n{}", imp::devices(k).iter().map(|(_, d)| format!("  {d}")).collect::<Vec<_>>().join("\n"))).collect::<Vec<_>>().join("\n")
    }
    #[cfg(not(windows))]
    String::new()
}
