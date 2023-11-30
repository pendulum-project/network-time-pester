use libc::c_int;
use pete::{Ptracer, Registers, Restart, Stop, Tracee};
use std::fmt::{Debug, Formatter};
use std::mem::{size_of, MaybeUninit};
use std::os::fd::RawFd;
use std::process::Command;
use std::time::Duration;
use syscalls::{SyscallArgs, Sysno};

fn main() {
    let mut tracer = Ptracer::new();
    *tracer.poll_delay_mut() = Duration::from_millis(1);

    let mut cmd = Command::new("/home/tamme/Projects/ntpd-rs/target/release/ntp-daemon");
    cmd.args(["-c", "/home/tamme/Projects/ntpd-rs/ntp.server.toml"]);

    tracer.spawn(cmd).expect("Can spawn process");

    while let Some(mut tracee) = tracer.wait().expect("wait never fails") {
        if matches!(tracee.stop, Stop::SyscallExit) {
            let regs = tracee.registers().unwrap();
            let sysno = Sysno::new(regs.orig_rax as usize).unwrap();

            match sysno {
                Sysno::recvmsg => handle_recvmsg(&mut tracee),
                Sysno::clock_adjtime => handle_adjtime(&mut tracee),
                Sysno::fcntl
                | Sysno::sigaltstack
                | Sysno::unlink
                | Sysno::mprotect
                | Sysno::getrandom
                | Sysno::rt_sigprocmask
                | Sysno::set_robust_list
                | Sysno::execve
                | Sysno::poll
                | Sysno::clone3
                | Sysno::pread64
                | Sysno::mmap
                | Sysno::munmap
                | Sysno::bind
                | Sysno::statx
                | Sysno::epoll_ctl
                | Sysno::prlimit64
                | Sysno::epoll_create1
                | Sysno::eventfd2
                | Sysno::prctl
                | Sysno::set_tid_address
                | Sysno::futex
                | Sysno::arch_prctl
                | Sysno::access
                | Sysno::newfstatat
                | Sysno::rt_sigaction
                | Sysno::write
                | Sysno::setsockopt
                | Sysno::openat
                | Sysno::socket
                | Sysno::brk
                | Sysno::close
                | Sysno::rseq
                | Sysno::sched_getaffinity
                | Sysno::read
                | Sysno::epoll_wait
                | Sysno::sendto => {}
                other => {
                    panic!("don't know what to do with syscall: {other}")
                }
            }
        }

        tracer.restart(tracee, Restart::Syscall).unwrap();
    }
}

fn handle_adjtime(tracee: &mut Tracee) {
    let pid = tracee.pid;
    let adj_time = AdjTime::from_tracee(tracee).unwrap();
    println!("[{pid}] {adj_time:?} = {:?}", adj_time.result);
}

fn handle_recvmsg(tracee: &mut Tracee) {
    let pid = tracee.pid;
    let recvmsg = RecvMsg::from_tracee(tracee).unwrap();

    println!("[{pid}] {recvmsg:?} = {:?}", recvmsg.result);
    if recvmsg.result.is_ok() && recvmsg.ctrl.is_some() {
        let ctrl = recvmsg.ctrl.unwrap();
        assert!(matches!(ctrl, ControlMsg::ScmTimeStamping(_)));
        let time = libc::timespec {
            tv_sec: -86400 * (70 * 365 + 17), // NTP era start
            tv_nsec: 0,
        };
        let buf: [u8; size_of::<libc::timespec>()] = unsafe { std::mem::transmute(time) };
        tracee
            .write_memory(
                recvmsg.header.msg_control as u64 + size_of::<libc::cmsghdr>() as u64,
                buf.as_slice(),
            )
            .unwrap();
    }
}

struct SysCall {
    pub no: Sysno,
    pub args: SyscallArgs,
    pub result: SysCallResult,
}

pub type SysCallResult = Result<usize, nix::errno::Errno>;

impl SysCall {
    pub fn new(regs: Registers) -> Option<Self> {
        let result = match regs.rax as isize {
            i @ 0.. => Ok(i as usize),
            i @ ..=-1 => Err(nix::errno::from_i32(-i as i32)),
        };

        Some(Self {
            no: Sysno::new(regs.orig_rax as usize)?,
            args: Self::regs_to_args(regs),
            result,
        })
    }

    fn regs_to_args(regs: Registers) -> SyscallArgs {
        SyscallArgs {
            arg0: regs.rdi as usize,
            arg1: regs.rsi as usize,
            arg2: regs.rdx as usize,
            arg3: regs.r10 as usize,
            arg4: regs.r8 as usize,
            arg5: regs.r9 as usize,
        }
    }
}

unsafe fn read_from_tracee<T: Copy>(tracee: &mut Tracee, addr: usize) -> T {
    let mut val: MaybeUninit<T> = MaybeUninit::zeroed();
    tracee
        .read_memory_mut(
            addr as u64,
            std::slice::from_raw_parts_mut(val.as_mut_ptr() as *mut u8, size_of::<T>()),
        )
        .unwrap();
    val.assume_init()
}

#[derive(Debug)]
struct AdjTime {
    clock_id: ClockId,
    timex: libc::timex,
    result: SysCallResult,
}

impl AdjTime {
    pub fn from_tracee(tracee: &mut Tracee) -> Option<Self> {
        let syscall = SysCall::new(tracee.registers().ok()?)?;
        if syscall.no != Sysno::clock_adjtime {
            return None;
        }

        Some(Self {
            clock_id: ClockId::from(syscall.args.arg0),
            timex: unsafe { read_from_tracee(tracee, syscall.args.arg1) },
            result: syscall.result,
        })
    }
}

#[derive(Debug)]
enum ClockId {
    Other(usize),
}

impl From<usize> for ClockId {
    fn from(value: usize) -> Self {
        Self::Other(value)
    }
}

struct RecvMsg {
    fd: RawFd,
    header: libc::msghdr,
    flags: c_int,
    result: SysCallResult,

    msg_control: Vec<u8>,
    ctrl: Option<ControlMsg>,
}

impl RecvMsg {
    fn from_tracee(tracee: &mut Tracee) -> Option<Self> {
        let syscall = SysCall::new(tracee.registers().ok()?)?;
        if syscall.no != Sysno::recvmsg {
            return None;
        }

        let header: libc::msghdr = unsafe { read_from_tracee(tracee, syscall.args.arg1) };

        let msg_control = tracee
            .read_memory(header.msg_control as _, header.msg_controllen as _)
            .unwrap();

        let ctrl = ControlMsg::parse(msg_control.as_slice());

        Some(Self {
            fd: syscall.args.arg0 as _,
            header,
            flags: syscall.args.arg2 as _,
            result: syscall.result,
            msg_control,
            ctrl,
        })
    }
}

impl Debug for RecvMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecvMsg")
            .field("fd", &self.fd)
            .field("result", &self.result)
            .field("ctrl", &self.ctrl)
            .finish()
    }
}

#[derive(Debug)]
enum ControlMsg {
    ScmTimeStamping([libc::timespec; 3]),
    ScmTimeStampNs(libc::timespec),
    ScmTimeStamp(libc::timeval),
    Other(libc::cmsghdr),
}

impl ControlMsg {
    fn parse(data: &[u8]) -> Option<Self> {
        assert!(data.len() >= size_of::<libc::cmsghdr>());
        let ctrl_hdr = unsafe { std::ptr::read_unaligned(data.as_ptr() as *const libc::cmsghdr) };
        let hdr_size = size_of::<libc::cmsghdr>();
        if ctrl_hdr.cmsg_len == 0 {
            return None;
        }
        let ctrl_data = &data[hdr_size..ctrl_hdr.cmsg_len];

        Some(match (ctrl_hdr.cmsg_level, ctrl_hdr.cmsg_type) {
            (libc::SOL_SOCKET, libc::SCM_TIMESTAMPING) => {
                type Record = [libc::timespec; 3];
                assert_eq!(ctrl_data.len(), size_of::<Record>());
                let record =
                    unsafe { std::ptr::read_unaligned(ctrl_data.as_ptr() as *const Record) };
                Self::ScmTimeStamping(record)
            }
            (libc::SOL_SOCKET, libc::SCM_TIMESTAMPNS) => {
                type Record = libc::timespec;
                assert_eq!(ctrl_data.len(), size_of::<Record>());
                let record =
                    unsafe { std::ptr::read_unaligned(ctrl_data.as_ptr() as *const Record) };
                Self::ScmTimeStampNs(record)
            }
            (libc::SOL_SOCKET, libc::SCM_TIMESTAMP) => {
                type Record = libc::timeval;
                assert_eq!(ctrl_data.len(), size_of::<Record>());
                let record =
                    unsafe { std::ptr::read_unaligned(ctrl_data.as_ptr() as *const Record) };
                Self::ScmTimeStamp(record)
            }
            _ => Self::Other(ctrl_hdr),
        })
    }
}
