use libc::{c_void, close, recvfrom, sendto, sockaddr, sockaddr_nl, socket};
use std::io;
use std::mem::{size_of, zeroed};
use std::os::fd::RawFd;
use std::ptr;

const SOCK_DIAG_BY_FAMILY: u16 = 20;
const ALL_TCP_STATES: u32 = 0xffffffff;
const TCP_ESTABLISHED: u32 = 1;
const NLMSG_HDRLEN: usize = size_of::<libc::nlmsghdr>();

/// ---- 与内核对齐的 C 结构体 ----

// from linux/inet_diag.h
#[repr(C)]
#[derive(Clone, Copy)]
struct InetDiagSockId {
    idiag_sport: u16,
    idiag_dport: u16,
    idiag_src: [u32; 4], // 足够容纳 IPv6（IPv4 只用 idiag_src[0]）
    idiag_dst: [u32; 4],
    idiag_if: u32,
    idiag_cookie: [u32; 2],
}

// from linux/inet_diag.h
#[repr(C)]
#[derive(Clone, Copy)]
struct InetDiagReqV2 {
    family: u8,
    protocol: u8,
    ext: u8,
    pad: u8,
    states: u32,
    id: InetDiagSockId,
}

/// 入口：按协议统计连接消息条数
pub fn connections_count_with_protocol(family: u8, protocol: u8) -> io::Result<u64> {
    // 构造 netlink header
    let hdr = libc::nlmsghdr {
        nlmsg_len: 0, // 先置 0，serialize 时回填
        nlmsg_type: SOCK_DIAG_BY_FAMILY,
        nlmsg_flags: (libc::NLM_F_DUMP | libc::NLM_F_REQUEST) as u16,
        nlmsg_seq: 0,
        nlmsg_pid: 0,
    };

    // 构造 inet_diag_req_v2
    let mut req = InetDiagReqV2 {
        family,
        protocol,
        ext: 0,
        pad: 0,
        states: ALL_TCP_STATES,
        id: InetDiagSockId {
            idiag_sport: 0,
            idiag_dport: 0,
            idiag_src: [0; 4],
            idiag_dst: [0; 4],
            idiag_if: 0,
            idiag_cookie: [0; 2],
        },
    };

    // TCP 只查询 ESTABLISHED 状态的
    if protocol == libc::IPPROTO_TCP as u8 {
        req.states = 1 << TCP_ESTABLISHED;
    }

    // 序列化成一条 Netlink 消息（header + payload）
    let msg = serialize_netlink_message(&hdr, &req)?;

    // 发送并只统计返回消息条数
    netlink_inet_diag_only_count(&msg)
}

fn netlink_inet_diag_only_count(request: &[u8]) -> io::Result<u64> {
    let fd = unsafe { socket(libc::AF_NETLINK, libc::SOCK_RAW, libc::NETLINK_SOCK_DIAG) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let _guard = FdGuard(fd);

    let mut addr: sockaddr_nl = unsafe { zeroed() };
    addr.nl_family = libc::AF_NETLINK as u16;
    addr.nl_pid = 0;
    addr.nl_groups = 0;

    // sendto
    let ret = unsafe {
        sendto(
            fd,
            request.as_ptr() as *const c_void,
            request.len(),
            0,
            &addr as *const sockaddr_nl as *const sockaddr,
            size_of::<sockaddr_nl>() as u32,
        )
    };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    // 准备读 buffer
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
    let mut buf: Vec<u8> = vec![0u8; page_size];

    let mut total_count: u64 = 0;

    loop {
        // 每次用整块 buf，当次有效长度是 nr
        let nr = unsafe {
            recvfrom(
                fd,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if nr < 0 {
            return Err(io::Error::last_os_error());
        }
        let nr = nr as usize;
        if nr < NLMSG_HDRLEN {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        let slice = &buf[..nr];

        let (count, done) = count_netlink_messages(slice)?;
        total_count += count;
        if done {
            break;
        }
    }

    Ok(total_count)
}

/// 只数本批次 buffer 里的 netlink 消息条数；遇到 DONE/ERROR 返回 done=true
fn count_netlink_messages(mut b: &[u8]) -> io::Result<(u64, bool)> {
    let mut msgs: u64 = 0;
    let mut done = false;

    while b.len() >= NLMSG_HDRLEN {
        let (dlen, at_end) = netlink_message_header(b)?;
        msgs += 1;
        if at_end {
            done = true;
            break;
        }
        b = &b[dlen..];
    }

    Ok((msgs, done))
}

/// 解析当前切片的 nlmsghdr，返回（对齐后的本条长度，是否 DONE/ERROR）
fn netlink_message_header(b: &[u8]) -> io::Result<(usize, bool)> {
    if b.len() < NLMSG_HDRLEN {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }

    // 安全地读出头（按本机字节序）
    let h = unsafe { &*(b.as_ptr() as *const libc::nlmsghdr) };
    let len = h.nlmsg_len as usize;
    let l = nlm_align_of(len as i32) as usize;

    if len < NLMSG_HDRLEN || l > b.len() {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }

    if h.nlmsg_type == libc::NLMSG_DONE as u16 || h.nlmsg_type == libc::NLMSG_ERROR as u16 {
        return Ok((l, true));
    }

    Ok((l, false))
}

/// 对齐到 4 字节
#[inline]
fn nlm_align_of(msglen: i32) -> i32 {
    (msglen + libc::NLA_ALIGNTO - 1) & !(libc::NLA_ALIGNTO - 1)
}

/// 将 (header, payload) 序列化为一条 Netlink 消息（回填 header.len）
fn serialize_netlink_message(hdr: &libc::nlmsghdr, req: &InetDiagReqV2) -> io::Result<Vec<u8>> {
    let total = NLMSG_HDRLEN + size_of::<InetDiagReqV2>();
    let mut msg = vec![0u8; total];

    // 写 header（先拷一份，回填 nlmsg_len）
    let mut h = *hdr;
    h.nlmsg_len = total as u32;

    unsafe {
        // header
        ptr::copy_nonoverlapping(
            &h as *const libc::nlmsghdr as *const u8,
            msg.as_mut_ptr(),
            NLMSG_HDRLEN,
        );
        // payload
        ptr::copy_nonoverlapping(
            req as *const InetDiagReqV2 as *const u8,
            msg.as_mut_ptr().add(NLMSG_HDRLEN),
            size_of::<InetDiagReqV2>(),
        );
    }

    Ok(msg)
}

/// 简易 FD 守卫
struct FdGuard(RawFd);
impl Drop for FdGuard {
    fn drop(&mut self) {
        unsafe { close(self.0) };
    }
}
