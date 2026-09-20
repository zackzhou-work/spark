/// 读取 Claude 核心进程环境变量里的宿主会话 ID，与桌面端会话 JSON 的 sessionId 一一对应
pub fn host_session_id(pid: u32) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        const KEY: &[u8] = b"CLAUDE_CODE_HOST_SESSION_ID=";
        read_proc_args(pid)?
            .split(|b| *b == 0)
            .find(|s| s.starts_with(KEY))
            .map(|s| String::from_utf8_lossy(&s[KEY.len()..]).into_owned())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

/// KERN_PROCARGS2 返回 argc、可执行路径、参数与环境变量，以 NUL 分隔
#[cfg(target_os = "macos")]
fn read_proc_args(pid: u32) -> Option<Vec<u8>> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
    let mut size: libc::size_t = 0;
    unsafe {
        let query = libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if query != 0 || size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size];
        let fetch = libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if fetch != 0 {
            return None;
        }
        buf.truncate(size);
        Some(buf)
    }
}
