//! # Cross-Architecture Syscall ABI Description and Wrapper
//!
//! Describe the syscall ABI for x86_64, aarch64, and riscv64 on Linux by filling in struct fields.
//! Also implement real syscall invocations on the current platform via conditional compilation.
//!
//! ## Background
//!
//! Different CPU architectures use different instructions and registers to trigger system calls:
//!
//! | Arch     | Instruction | Syscall ID Reg | Return Reg | Argument Registers              |
//! |----------|-------------|----------------|------------|---------------------------------|
//! | x86_64   | `syscall`   | rax            | rax        | rdi, rsi, rdx, r10, r8, r9     |
//! | aarch64  | `svc #0`    | x8             | x0         | x0, x1, x2, x3, x4, x5        |
//! | riscv64  | `ecall`     | a7             | a0         | a0, a1, a2, a3, a4, a5         |
//!
//! ## Task
//!
//! 1. Implement `x86_64_abi()`, `aarch64_abi()`, `riscv64_abi()` — return structs describing each arch's ABI
//! 2. (Conditional compilation) Implement real `syscall3` inline assembly on the current platform
//! 3. Build `sys_write` / `sys_read` / `sys_close` / `sys_exit` on top of `syscall3`
//!
//! ## Hints
//!
//! - Linux syscall numbers differ across architectures; x86_64 vs aarch64/riscv64 are quite different
//! - The x86_64 `syscall` instruction clobbers the rcx and r11 registers
//! - aarch64 and riscv64 share the unified syscall number table (from asm-generic)

#![cfg_attr(not(test), no_std)]

/// Describes a Linux Syscall ABI for a specific architecture
pub struct SyscallABI {
    /// Architecture name: "x86_64", "aarch64", "riscv64"
    pub arch: &'static str,
    /// Instruction that triggers the syscall: "syscall", "svc #0", "ecall"
    pub instruction: &'static str,
    /// Register holding the syscall number
    pub id_reg: &'static str,
    /// Register holding the return value
    pub ret_reg: &'static str,
    /// Argument registers (in order)
    pub arg_regs: &'static [&'static str],
    /// Registers additionally clobbered by the syscall instruction
    pub clobbered: &'static [&'static str],
    /// write syscall number
    pub sys_write: usize,
    /// read syscall number
    pub sys_read: usize,
    /// close syscall number
    pub sys_close: usize,
    /// exit syscall number
    pub sys_exit: usize,
}

/// Return the x86_64 Linux syscall ABI description
pub fn x86_64_abi() -> SyscallABI {
    // TODO: Fill in the x86_64 syscall ABI
    // Hint: x86_64 uses the "syscall" instruction, syscall number in rax
    SyscallABI {
        arch: "x86_64",
        instruction: "syscall",
        id_reg: "rax",
        ret_reg: "rax",
        arg_regs: &["rdi", "rsi", "rdx", "r10", "r8", "r9"],
        clobbered: &["rcx", "r11"],
        sys_write: 1,
        sys_read: 0,
        sys_close: 3,
        sys_exit: 60,
    }
}

/// Return the aarch64 Linux syscall ABI description
pub fn aarch64_abi() -> SyscallABI {
    // TODO: Fill in the aarch64 syscall ABI
    // Hint: aarch64 uses the "svc #0" instruction, syscall number in x8
    SyscallABI {
        arch: "aarch64",
        instruction: "svc #0",
        id_reg: "x8",
        ret_reg: "x0",
        arg_regs: &["x0", "x1", "x2", "x3", "x4", "x5"],
        clobbered: &[],
        sys_write: 64,
        sys_read: 63,
        sys_close: 57,
        sys_exit: 93,
    }
}

/// Return the riscv64 Linux syscall ABI description
pub fn riscv64_abi() -> SyscallABI {
    // TODO: Fill in the riscv64 syscall ABI
    // Hint: riscv64 uses the "ecall" instruction, syscall number in a7
    SyscallABI {
        arch: "riscv64",
        instruction: "ecall",
        id_reg: "a7",
        ret_reg: "a0",
        arg_regs: &["a0", "a1", "a2", "a3", "a4", "a5"],
        clobbered: &[],
        sys_write: 64,
        sys_read: 63,
        sys_close: 57,
        sys_exit: 93,
    }
}

// ============================================================
// Real syscall implementation (conditional compilation, only active on matching platform)
// ============================================================

/// Issue a Linux syscall with up to 3 arguments.
///
/// # Safety
/// The caller must ensure the syscall number and arguments are valid.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub unsafe fn syscall3(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    // TODO: Implement x86_64 syscall using core::arch::asm!
    // Hints:
    //   - "syscall" instruction
    //   - inlateout("rax") id => ret
    //   - in("rdi") arg0, in("rsi") arg1, in("rdx") arg2
    //   - out("rcx") _, out("r11") _
    let mut ret: isize;
    core::arch::asm! (
        "syscall",
        inlateout("rax") id => ret,
        in("rdi") arg0, in("rsi") arg1, in("rdx") arg2,
        out("rcx") _, out("r11") _
    );
    ret
}

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub unsafe fn syscall3(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    // TODO: Implement aarch64 syscall using core::arch::asm!
    // Hints:
    //   - "svc #0" instruction
    //   - in("x8") id
    //   - inlateout("x0") arg0 => ret
    //   - in("x1") arg1, in("x2") arg2
    let mut ret: isize;
    core::arch::asm! (
        "svc #0",
        in("x8") id,
        inlateout("x0") arg0 => ret,
        in("x1") arg1, in("x2") arg2
    );
    ret
}

// Non-Linux platforms: provide a stub so the code compiles
#[cfg(not(target_os = "linux"))]
pub unsafe fn syscall3(_id: usize, _arg0: usize, _arg1: usize, _arg2: usize) -> isize {
    panic!("syscall3 is only available on Linux")
}

// Platform-specific write syscall number
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_WRITE: usize = 1;
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_READ: usize = 0;
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_CLOSE: usize = 3;
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_EXIT: usize = 60;

#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_WRITE: usize = 64;
#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_READ: usize = 63;
#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_CLOSE: usize = 57;
#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_EXIT: usize = 93;

// Fallback for other architectures (not actually used, just for compilation)
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_WRITE: usize = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_READ: usize = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_CLOSE: usize = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_EXIT: usize = 0;

/// Write data from `buf` to file descriptor `fd`.
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    // TODO: Call syscall3 to implement write
    unsafe { syscall3(NATIVE_SYS_WRITE, fd, buf.as_ptr() as usize, buf.len()) }
}

/// Read data from file descriptor `fd` into `buf`.
pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    // TODO: Call syscall3 to implement read
    unsafe { syscall3(NATIVE_SYS_READ, fd, buf.as_mut_ptr() as usize, buf.len()) }
}

/// Close file descriptor `fd`.
pub fn sys_close(fd: usize) -> isize {
    // TODO: Call syscall3 to implement close
    unsafe { syscall3(NATIVE_SYS_CLOSE, fd, 0, 0) }
}

/// Terminate the current process.
pub fn sys_exit(code: i32) -> ! {
    // TODO: Call syscall3 to implement exit
    unsafe {
        syscall3(NATIVE_SYS_EXIT, code as usize, 0, 0);
    }
    unreachable!()
}

// ============================================================
// Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ---- ABI knowledge tests (run on any platform) ----

    #[test]
    fn test_x86_64_instruction() {
        let abi = x86_64_abi();
        assert_eq!(abi.arch, "x86_64");
        assert_eq!(abi.instruction, "syscall");
    }

    #[test]
    fn test_x86_64_registers() {
        let abi = x86_64_abi();
        assert_eq!(abi.id_reg, "rax");
        assert_eq!(abi.ret_reg, "rax");
        assert_eq!(
            abi.arg_regs,
            &["rdi", "rsi", "rdx", "r10", "r8", "r9"],
            "x86_64 argument register order is incorrect"
        );
    }

    #[test]
    fn test_x86_64_clobbered() {
        let abi = x86_64_abi();
        assert!(
            abi.clobbered.contains(&"rcx") && abi.clobbered.contains(&"r11"),
            "x86_64 syscall clobbers rcx and r11"
        );
    }

    #[test]
    fn test_x86_64_syscall_numbers() {
        let abi = x86_64_abi();
        assert_eq!(abi.sys_write, 1);
        assert_eq!(abi.sys_read, 0);
        assert_eq!(abi.sys_close, 3);
        assert_eq!(abi.sys_exit, 60);
    }

    #[test]
    fn test_aarch64_instruction() {
        let abi = aarch64_abi();
        assert_eq!(abi.arch, "aarch64");
        assert_eq!(abi.instruction, "svc #0");
    }

    #[test]
    fn test_aarch64_registers() {
        let abi = aarch64_abi();
        assert_eq!(abi.id_reg, "x8");
        assert_eq!(abi.ret_reg, "x0");
        assert_eq!(
            abi.arg_regs,
            &["x0", "x1", "x2", "x3", "x4", "x5"],
            "aarch64 argument register order is incorrect"
        );
    }

    #[test]
    fn test_aarch64_clobbered() {
        let abi = aarch64_abi();
        assert!(
            abi.clobbered.is_empty(),
            "aarch64 svc does not clobber additional registers"
        );
    }

    #[test]
    fn test_aarch64_syscall_numbers() {
        let abi = aarch64_abi();
        assert_eq!(abi.sys_write, 64);
        assert_eq!(abi.sys_read, 63);
        assert_eq!(abi.sys_close, 57);
        assert_eq!(abi.sys_exit, 93);
    }

    #[test]
    fn test_riscv64_instruction() {
        let abi = riscv64_abi();
        assert_eq!(abi.arch, "riscv64");
        assert_eq!(abi.instruction, "ecall");
    }

    #[test]
    fn test_riscv64_registers() {
        let abi = riscv64_abi();
        assert_eq!(abi.id_reg, "a7");
        assert_eq!(abi.ret_reg, "a0");
        assert_eq!(
            abi.arg_regs,
            &["a0", "a1", "a2", "a3", "a4", "a5"],
            "riscv64 argument register order is incorrect"
        );
    }

    #[test]
    fn test_riscv64_clobbered() {
        let abi = riscv64_abi();
        assert!(
            abi.clobbered.is_empty(),
            "riscv64 ecall does not clobber additional registers"
        );
    }

    #[test]
    fn test_riscv64_syscall_numbers() {
        let abi = riscv64_abi();
        assert_eq!(abi.sys_write, 64);
        assert_eq!(abi.sys_read, 63);
        assert_eq!(abi.sys_close, 57);
        assert_eq!(abi.sys_exit, 93);
    }

    #[test]
    fn test_aarch64_riscv64_share_numbers() {
        let aarch64 = aarch64_abi();
        let riscv64 = riscv64_abi();
        assert_eq!(
            aarch64.sys_write, riscv64.sys_write,
            "aarch64 and riscv64 share asm-generic syscall numbers"
        );
        assert_eq!(aarch64.sys_read, riscv64.sys_read);
        assert_eq!(aarch64.sys_close, riscv64.sys_close);
        assert_eq!(aarch64.sys_exit, riscv64.sys_exit);
    }

    // ---- Real syscall tests (only run on Linux) ----

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;

        #[test]
        fn test_sys_write_stdout() {
            let msg = b"[syscall_wrapper] sys_write test\n";
            let ret = sys_write(1, msg);
            assert_eq!(
                ret,
                msg.len() as isize,
                "sys_write should return bytes written"
            );
        }

        #[test]
        fn test_sys_write_stderr() {
            let msg = b"[syscall_wrapper] stderr test\n";
            let ret = sys_write(2, msg);
            assert_eq!(ret, msg.len() as isize);
        }

        #[test]
        fn test_sys_write_invalid_fd() {
            let ret = sys_write(999, b"hello");
            assert!(ret < 0, "invalid fd should return negative, got {ret}");
        }

        #[test]
        fn test_sys_close_invalid_fd() {
            let ret = sys_close(999);
            assert!(ret < 0, "closing invalid fd should return negative");
        }
    }
}
