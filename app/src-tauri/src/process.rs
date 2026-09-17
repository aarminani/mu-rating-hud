const GAME_EXE: &str = "Polaris-Win64-Shipping.exe";

const GAME_EXE_STEM: &str = "Polaris-Win64-Shipping";

pub fn game_running() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {GAME_EXE}"), "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(GAME_EXE_STEM))
        .unwrap_or(false)
}

pub fn game_has_focus() -> bool {
    use std::os::windows::ffi::OsStringExt;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn QueryFullProcessImageNameW(handle: isize, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return false;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return false;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) != 0;
        CloseHandle(handle);
        if !ok {
            return false;
        }
        std::ffi::OsString::from_wide(&buf[..len as usize])
            .to_string_lossy()
            .contains(GAME_EXE_STEM)
    }
}
