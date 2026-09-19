const GAME_EXE: &str = "Polaris-Win64-Shipping.exe";

const GAME_EXE_STEM: &str = "Polaris-Win64-Shipping";

pub fn game_running() -> bool {
    matches!(game_check(), GameCheck::Running(_))
}

pub enum GameCheck {
    Running(Option<u32>),
    Stopped,
    Unknown,
}

pub fn game_check() -> GameCheck {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {GAME_EXE}"), "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(out) => classify(&String::from_utf8_lossy(&out.stdout), out.status.success()),
        Err(_) => GameCheck::Unknown,
    }
}

pub fn classify(stdout: &str, success: bool) -> GameCheck {
    if let Some(line) = stdout.lines().find(|l| l.contains(GAME_EXE_STEM)) {
        return GameCheck::Running(line.split_whitespace().nth(1).and_then(|p| p.parse().ok()));
    }
    if success && !stdout.trim().is_empty() {
        GameCheck::Stopped
    } else {
        GameCheck::Unknown
    }
}

#[derive(Default)]
pub struct GameSeen {
    pub running: bool,
    pid: Option<u32>,
    gone: u32,
}

pub const GAME_GONE_CHECKS: u32 = 2;

impl GameSeen {
    pub fn observe(&mut self, check: GameCheck) -> bool {
        match check {
            GameCheck::Unknown => false,
            GameCheck::Running(pid) => {
                let launched = !self.running && (pid.is_none() || pid != self.pid);
                self.running = true;
                self.gone = 0;
                if pid.is_some() {
                    self.pid = pid;
                }
                launched
            }
            GameCheck::Stopped => {
                self.gone += 1;
                if self.gone >= GAME_GONE_CHECKS {
                    self.running = false;
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNNING: &str = "\r\nPolaris-Win64-Shipping.ex    11872 Console                    1  5,269,432 K\r\n";
    const NONE: &str = "INFO: No tasks are running which match the specified criteria.\r\n";

    fn run(pid: u32) -> GameCheck {
        GameCheck::Running(Some(pid))
    }

    #[test]
    fn reads_the_process_id_out_of_a_match() {
        assert!(matches!(classify(RUNNING, true), GameCheck::Running(Some(11872))));
    }

    #[test]
    fn no_match_is_stopped_in_any_language() {
        assert!(matches!(classify(NONE, true), GameCheck::Stopped));
        assert!(matches!(classify("INFORMATIONS : Aucune tâche en cours.\r\n", true), GameCheck::Stopped));
    }

    #[test]
    fn a_failed_or_empty_answer_is_unknown_not_stopped() {
        assert!(matches!(classify("", true), GameCheck::Unknown));
        assert!(matches!(classify("\r\n", true), GameCheck::Unknown));
        assert!(matches!(classify(NONE, false), GameCheck::Unknown));
    }

    #[test]
    fn tekken_starting_is_a_launch_once() {
        let mut g = GameSeen::default();
        assert!(g.observe(run(100)), "helper up first, then Tekken: that is a launch");
        assert!(!g.observe(run(100)), "still the same game");
    }

    #[test]
    fn a_failed_check_mid_match_is_not_a_relaunch() {
        let mut g = GameSeen::default();
        g.observe(run(100));
        assert!(!g.observe(GameCheck::Unknown));
        assert!(g.running, "a failed check changes nothing");
        assert!(!g.observe(run(100)));
    }

    #[test]
    fn one_clean_miss_is_not_enough_to_count_as_closed() {
        let mut g = GameSeen::default();
        g.observe(run(100));
        g.observe(GameCheck::Stopped);
        assert!(g.running);
        assert!(!g.observe(run(100)));
    }

    #[test]
    fn the_same_process_is_never_a_launch_even_after_it_looked_gone() {
        let mut g = GameSeen::default();
        g.observe(run(100));
        g.observe(GameCheck::Stopped);
        g.observe(GameCheck::Stopped);
        assert!(!g.running);
        assert!(!g.observe(run(100)), "same pid: the checks lied, the game never closed");
    }

    #[test]
    fn a_real_restart_is_a_launch() {
        let mut g = GameSeen::default();
        g.observe(run(100));
        g.observe(GameCheck::Stopped);
        g.observe(GameCheck::Stopped);
        assert!(g.observe(run(200)), "closed and reopened: new pid");
    }
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
