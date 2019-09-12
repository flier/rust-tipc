/* automatically generated by rust-bindgen */

pub const __BITS_PER_LONG: u32 = 64;
pub const __FD_SETSIZE: u32 = 1024;
pub const FIOSETOWN: u32 = 35073;
pub const SIOCSPGRP: u32 = 35074;
pub const FIOGETOWN: u32 = 35075;
pub const SIOCGPGRP: u32 = 35076;
pub const SIOCATMARK: u32 = 35077;
pub const SIOCGSTAMP: u32 = 35078;
pub const SIOCGSTAMPNS: u32 = 35079;
pub const SOCK_IOC_TYPE: u32 = 137;
pub const SIOCADDRT: u32 = 35083;
pub const SIOCDELRT: u32 = 35084;
pub const SIOCRTMSG: u32 = 35085;
pub const SIOCGIFNAME: u32 = 35088;
pub const SIOCSIFLINK: u32 = 35089;
pub const SIOCGIFCONF: u32 = 35090;
pub const SIOCGIFFLAGS: u32 = 35091;
pub const SIOCSIFFLAGS: u32 = 35092;
pub const SIOCGIFADDR: u32 = 35093;
pub const SIOCSIFADDR: u32 = 35094;
pub const SIOCGIFDSTADDR: u32 = 35095;
pub const SIOCSIFDSTADDR: u32 = 35096;
pub const SIOCGIFBRDADDR: u32 = 35097;
pub const SIOCSIFBRDADDR: u32 = 35098;
pub const SIOCGIFNETMASK: u32 = 35099;
pub const SIOCSIFNETMASK: u32 = 35100;
pub const SIOCGIFMETRIC: u32 = 35101;
pub const SIOCSIFMETRIC: u32 = 35102;
pub const SIOCGIFMEM: u32 = 35103;
pub const SIOCSIFMEM: u32 = 35104;
pub const SIOCGIFMTU: u32 = 35105;
pub const SIOCSIFMTU: u32 = 35106;
pub const SIOCSIFNAME: u32 = 35107;
pub const SIOCSIFHWADDR: u32 = 35108;
pub const SIOCGIFENCAP: u32 = 35109;
pub const SIOCSIFENCAP: u32 = 35110;
pub const SIOCGIFHWADDR: u32 = 35111;
pub const SIOCGIFSLAVE: u32 = 35113;
pub const SIOCSIFSLAVE: u32 = 35120;
pub const SIOCADDMULTI: u32 = 35121;
pub const SIOCDELMULTI: u32 = 35122;
pub const SIOCGIFINDEX: u32 = 35123;
pub const SIOGIFINDEX: u32 = 35123;
pub const SIOCSIFPFLAGS: u32 = 35124;
pub const SIOCGIFPFLAGS: u32 = 35125;
pub const SIOCDIFADDR: u32 = 35126;
pub const SIOCSIFHWBROADCAST: u32 = 35127;
pub const SIOCGIFCOUNT: u32 = 35128;
pub const SIOCGIFBR: u32 = 35136;
pub const SIOCSIFBR: u32 = 35137;
pub const SIOCGIFTXQLEN: u32 = 35138;
pub const SIOCSIFTXQLEN: u32 = 35139;
pub const SIOCETHTOOL: u32 = 35142;
pub const SIOCGMIIPHY: u32 = 35143;
pub const SIOCGMIIREG: u32 = 35144;
pub const SIOCSMIIREG: u32 = 35145;
pub const SIOCWANDEV: u32 = 35146;
pub const SIOCOUTQNSD: u32 = 35147;
pub const SIOCGSKNS: u32 = 35148;
pub const SIOCDARP: u32 = 35155;
pub const SIOCGARP: u32 = 35156;
pub const SIOCSARP: u32 = 35157;
pub const SIOCDRARP: u32 = 35168;
pub const SIOCGRARP: u32 = 35169;
pub const SIOCSRARP: u32 = 35170;
pub const SIOCGIFMAP: u32 = 35184;
pub const SIOCSIFMAP: u32 = 35185;
pub const SIOCADDDLCI: u32 = 35200;
pub const SIOCDELDLCI: u32 = 35201;
pub const SIOCGIFVLAN: u32 = 35202;
pub const SIOCSIFVLAN: u32 = 35203;
pub const SIOCBONDENSLAVE: u32 = 35216;
pub const SIOCBONDRELEASE: u32 = 35217;
pub const SIOCBONDSETHWADDR: u32 = 35218;
pub const SIOCBONDSLAVEINFOQUERY: u32 = 35219;
pub const SIOCBONDINFOQUERY: u32 = 35220;
pub const SIOCBONDCHANGEACTIVE: u32 = 35221;
pub const SIOCBRADDBR: u32 = 35232;
pub const SIOCBRDELBR: u32 = 35233;
pub const SIOCBRADDIF: u32 = 35234;
pub const SIOCBRDELIF: u32 = 35235;
pub const SIOCSHWTSTAMP: u32 = 35248;
pub const SIOCGHWTSTAMP: u32 = 35249;
pub const SIOCDEVPRIVATE: u32 = 35312;
pub const SIOCPROTOPRIVATE: u32 = 35296;
pub const TIPC_NODE_BITS: u32 = 12;
pub const TIPC_CLUSTER_BITS: u32 = 12;
pub const TIPC_ZONE_BITS: u32 = 8;
pub const TIPC_NODE_OFFSET: u32 = 0;
pub const TIPC_CLUSTER_OFFSET: u32 = 12;
pub const TIPC_ZONE_OFFSET: u32 = 24;
pub const TIPC_NODE_SIZE: u32 = 4095;
pub const TIPC_CLUSTER_SIZE: u32 = 4095;
pub const TIPC_ZONE_SIZE: u32 = 255;
pub const TIPC_NODE_MASK: u32 = 4095;
pub const TIPC_CLUSTER_MASK: u32 = 16773120;
pub const TIPC_ZONE_MASK: u32 = 4278190080;
pub const TIPC_ZONE_CLUSTER_MASK: u32 = 4294963200;
pub const TIPC_CFG_SRV: u32 = 0;
pub const TIPC_TOP_SRV: u32 = 1;
pub const TIPC_LINK_STATE: u32 = 2;
pub const TIPC_RESERVED_TYPES: u32 = 64;
pub const TIPC_ZONE_SCOPE: u32 = 1;
pub const TIPC_CLUSTER_SCOPE: u32 = 2;
pub const TIPC_NODE_SCOPE: u32 = 3;
pub const TIPC_MAX_USER_MSG_SIZE: u32 = 66000;
pub const TIPC_LOW_IMPORTANCE: u32 = 0;
pub const TIPC_MEDIUM_IMPORTANCE: u32 = 1;
pub const TIPC_HIGH_IMPORTANCE: u32 = 2;
pub const TIPC_CRITICAL_IMPORTANCE: u32 = 3;
pub const TIPC_OK: u32 = 0;
pub const TIPC_ERR_NO_NAME: u32 = 1;
pub const TIPC_ERR_NO_PORT: u32 = 2;
pub const TIPC_ERR_NO_NODE: u32 = 3;
pub const TIPC_ERR_OVERLOAD: u32 = 4;
pub const TIPC_CONN_SHUTDOWN: u32 = 5;
pub const TIPC_SUB_PORTS: u32 = 1;
pub const TIPC_SUB_SERVICE: u32 = 2;
pub const TIPC_SUB_CANCEL: u32 = 4;
pub const TIPC_WAIT_FOREVER: i32 = -1;
pub const TIPC_PUBLISHED: u32 = 1;
pub const TIPC_WITHDRAWN: u32 = 2;
pub const TIPC_SUBSCR_TIMEOUT: u32 = 3;
pub const AF_TIPC: u32 = 30;
pub const PF_TIPC: u32 = 30;
pub const SOL_TIPC: u32 = 271;
pub const TIPC_ADDR_NAMESEQ: u32 = 1;
pub const TIPC_ADDR_MCAST: u32 = 1;
pub const TIPC_ADDR_NAME: u32 = 2;
pub const TIPC_ADDR_ID: u32 = 3;
pub const TIPC_ERRINFO: u32 = 1;
pub const TIPC_RETDATA: u32 = 2;
pub const TIPC_DESTNAME: u32 = 3;
pub const TIPC_IMPORTANCE: u32 = 127;
pub const TIPC_SRC_DROPPABLE: u32 = 128;
pub const TIPC_DEST_DROPPABLE: u32 = 129;
pub const TIPC_CONN_TIMEOUT: u32 = 130;
pub const TIPC_NODE_RECVQ_DEPTH: u32 = 131;
pub const TIPC_SOCK_RECVQ_DEPTH: u32 = 132;
pub const TIPC_MCAST_BROADCAST: u32 = 133;
pub const TIPC_MCAST_REPLICAST: u32 = 134;
pub const TIPC_GROUP_JOIN: u32 = 135;
pub const TIPC_GROUP_LEAVE: u32 = 136;
pub const TIPC_GROUP_LOOPBACK: u32 = 1;
pub const TIPC_GROUP_MEMBER_EVTS: u32 = 2;
pub const TIPC_MAX_MEDIA_NAME: u32 = 16;
pub const TIPC_MAX_IF_NAME: u32 = 16;
pub const TIPC_MAX_BEARER_NAME: u32 = 32;
pub const TIPC_MAX_LINK_NAME: u32 = 60;
pub const SIOCGETLINKNAME: u32 = 35296;
pub type __s8 = ::std::os::raw::c_schar;
pub type __u8 = ::std::os::raw::c_uchar;
pub type __s16 = ::std::os::raw::c_short;
pub type __u16 = ::std::os::raw::c_ushort;
pub type __s32 = ::std::os::raw::c_int;
pub type __u32 = ::std::os::raw::c_uint;
pub type __s64 = ::std::os::raw::c_longlong;
pub type __u64 = ::std::os::raw::c_ulonglong;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct __kernel_fd_set {
    pub fds_bits: [::std::os::raw::c_ulong; 16usize],
}
#[test]
fn bindgen_test_layout___kernel_fd_set() {
    assert_eq!(
        ::std::mem::size_of::<__kernel_fd_set>(),
        128usize,
        concat!("Size of: ", stringify!(__kernel_fd_set))
    );
    assert_eq!(
        ::std::mem::align_of::<__kernel_fd_set>(),
        8usize,
        concat!("Alignment of ", stringify!(__kernel_fd_set))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<__kernel_fd_set>())).fds_bits as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(__kernel_fd_set),
            "::",
            stringify!(fds_bits)
        )
    );
}
pub type __kernel_sighandler_t =
    ::std::option::Option<unsafe extern "C" fn(arg1: ::std::os::raw::c_int)>;
pub type __kernel_key_t = ::std::os::raw::c_int;
pub type __kernel_mqd_t = ::std::os::raw::c_int;
pub type __kernel_old_uid_t = ::std::os::raw::c_ushort;
pub type __kernel_old_gid_t = ::std::os::raw::c_ushort;
pub type __kernel_old_dev_t = ::std::os::raw::c_ulong;
pub type __kernel_long_t = ::std::os::raw::c_long;
pub type __kernel_ulong_t = ::std::os::raw::c_ulong;
pub type __kernel_ino_t = __kernel_ulong_t;
pub type __kernel_mode_t = ::std::os::raw::c_uint;
pub type __kernel_pid_t = ::std::os::raw::c_int;
pub type __kernel_ipc_pid_t = ::std::os::raw::c_int;
pub type __kernel_uid_t = ::std::os::raw::c_uint;
pub type __kernel_gid_t = ::std::os::raw::c_uint;
pub type __kernel_suseconds_t = __kernel_long_t;
pub type __kernel_daddr_t = ::std::os::raw::c_int;
pub type __kernel_uid32_t = ::std::os::raw::c_uint;
pub type __kernel_gid32_t = ::std::os::raw::c_uint;
pub type __kernel_size_t = __kernel_ulong_t;
pub type __kernel_ssize_t = __kernel_long_t;
pub type __kernel_ptrdiff_t = __kernel_long_t;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct __kernel_fsid_t {
    pub val: [::std::os::raw::c_int; 2usize],
}
#[test]
fn bindgen_test_layout___kernel_fsid_t() {
    assert_eq!(
        ::std::mem::size_of::<__kernel_fsid_t>(),
        8usize,
        concat!("Size of: ", stringify!(__kernel_fsid_t))
    );
    assert_eq!(
        ::std::mem::align_of::<__kernel_fsid_t>(),
        4usize,
        concat!("Alignment of ", stringify!(__kernel_fsid_t))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<__kernel_fsid_t>())).val as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(__kernel_fsid_t),
            "::",
            stringify!(val)
        )
    );
}
pub type __kernel_off_t = __kernel_long_t;
pub type __kernel_loff_t = ::std::os::raw::c_longlong;
pub type __kernel_time_t = __kernel_long_t;
pub type __kernel_clock_t = __kernel_long_t;
pub type __kernel_timer_t = ::std::os::raw::c_int;
pub type __kernel_clockid_t = ::std::os::raw::c_int;
pub type __kernel_caddr_t = *mut ::std::os::raw::c_char;
pub type __kernel_uid16_t = ::std::os::raw::c_ushort;
pub type __kernel_gid16_t = ::std::os::raw::c_ushort;
pub type __le16 = __u16;
pub type __be16 = __u16;
pub type __le32 = __u32;
pub type __be32 = __u32;
pub type __le64 = __u64;
pub type __be64 = __u64;
pub type __sum16 = __u16;
pub type __wsum = __u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct tipc_portid {
    pub ref_: __u32,
    pub node: __u32,
}
#[test]
fn bindgen_test_layout_tipc_portid() {
    assert_eq!(
        ::std::mem::size_of::<tipc_portid>(),
        8usize,
        concat!("Size of: ", stringify!(tipc_portid))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_portid>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_portid))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_portid>())).ref_ as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_portid),
            "::",
            stringify!(ref_)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_portid>())).node as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_portid),
            "::",
            stringify!(node)
        )
    );
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct tipc_name {
    pub type_: __u32,
    pub instance: __u32,
}
#[test]
fn bindgen_test_layout_tipc_name() {
    assert_eq!(
        ::std::mem::size_of::<tipc_name>(),
        8usize,
        concat!("Size of: ", stringify!(tipc_name))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_name>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_name))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_name>())).type_ as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_name),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_name>())).instance as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_name),
            "::",
            stringify!(instance)
        )
    );
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct tipc_name_seq {
    pub type_: __u32,
    pub lower: __u32,
    pub upper: __u32,
}
#[test]
fn bindgen_test_layout_tipc_name_seq() {
    assert_eq!(
        ::std::mem::size_of::<tipc_name_seq>(),
        12usize,
        concat!("Size of: ", stringify!(tipc_name_seq))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_name_seq>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_name_seq))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_name_seq>())).type_ as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_name_seq),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_name_seq>())).lower as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_name_seq),
            "::",
            stringify!(lower)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_name_seq>())).upper as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_name_seq),
            "::",
            stringify!(upper)
        )
    );
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct tipc_subscr {
    pub seq: tipc_name_seq,
    pub timeout: __u32,
    pub filter: __u32,
    pub usr_handle: [::std::os::raw::c_char; 8usize],
}
#[test]
fn bindgen_test_layout_tipc_subscr() {
    assert_eq!(
        ::std::mem::size_of::<tipc_subscr>(),
        28usize,
        concat!("Size of: ", stringify!(tipc_subscr))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_subscr>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_subscr))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_subscr>())).seq as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_subscr),
            "::",
            stringify!(seq)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_subscr>())).timeout as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_subscr),
            "::",
            stringify!(timeout)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_subscr>())).filter as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_subscr),
            "::",
            stringify!(filter)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_subscr>())).usr_handle as *const _ as usize },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_subscr),
            "::",
            stringify!(usr_handle)
        )
    );
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct tipc_event {
    pub event: __u32,
    pub found_lower: __u32,
    pub found_upper: __u32,
    pub port: tipc_portid,
    pub s: tipc_subscr,
}
#[test]
fn bindgen_test_layout_tipc_event() {
    assert_eq!(
        ::std::mem::size_of::<tipc_event>(),
        48usize,
        concat!("Size of: ", stringify!(tipc_event))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_event>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_event))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_event>())).event as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_event),
            "::",
            stringify!(event)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_event>())).found_lower as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_event),
            "::",
            stringify!(found_lower)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_event>())).found_upper as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_event),
            "::",
            stringify!(found_upper)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_event>())).port as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_event),
            "::",
            stringify!(port)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_event>())).s as *const _ as usize },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_event),
            "::",
            stringify!(s)
        )
    );
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct sockaddr_tipc {
    pub family: ::std::os::raw::c_ushort,
    pub addrtype: ::std::os::raw::c_uchar,
    pub scope: ::std::os::raw::c_schar,
    pub addr: sockaddr_tipc__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union sockaddr_tipc__bindgen_ty_1 {
    pub id: tipc_portid,
    pub nameseq: tipc_name_seq,
    pub name: sockaddr_tipc__bindgen_ty_1__bindgen_ty_1,
    _bindgen_union_align: [u32; 3usize],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct sockaddr_tipc__bindgen_ty_1__bindgen_ty_1 {
    pub name: tipc_name,
    pub domain: __u32,
}
#[test]
fn bindgen_test_layout_sockaddr_tipc__bindgen_ty_1__bindgen_ty_1() {
    assert_eq!(
        ::std::mem::size_of::<sockaddr_tipc__bindgen_ty_1__bindgen_ty_1>(),
        12usize,
        concat!(
            "Size of: ",
            stringify!(sockaddr_tipc__bindgen_ty_1__bindgen_ty_1)
        )
    );
    assert_eq!(
        ::std::mem::align_of::<sockaddr_tipc__bindgen_ty_1__bindgen_ty_1>(),
        4usize,
        concat!(
            "Alignment of ",
            stringify!(sockaddr_tipc__bindgen_ty_1__bindgen_ty_1)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<sockaddr_tipc__bindgen_ty_1__bindgen_ty_1>())).name as *const _
                as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc__bindgen_ty_1__bindgen_ty_1),
            "::",
            stringify!(name)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<sockaddr_tipc__bindgen_ty_1__bindgen_ty_1>())).domain as *const _
                as usize
        },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc__bindgen_ty_1__bindgen_ty_1),
            "::",
            stringify!(domain)
        )
    );
}
#[test]
fn bindgen_test_layout_sockaddr_tipc__bindgen_ty_1() {
    assert_eq!(
        ::std::mem::size_of::<sockaddr_tipc__bindgen_ty_1>(),
        12usize,
        concat!("Size of: ", stringify!(sockaddr_tipc__bindgen_ty_1))
    );
    assert_eq!(
        ::std::mem::align_of::<sockaddr_tipc__bindgen_ty_1>(),
        4usize,
        concat!("Alignment of ", stringify!(sockaddr_tipc__bindgen_ty_1))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<sockaddr_tipc__bindgen_ty_1>())).id as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc__bindgen_ty_1),
            "::",
            stringify!(id)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<sockaddr_tipc__bindgen_ty_1>())).nameseq as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc__bindgen_ty_1),
            "::",
            stringify!(nameseq)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<sockaddr_tipc__bindgen_ty_1>())).name as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc__bindgen_ty_1),
            "::",
            stringify!(name)
        )
    );
}
impl Default for sockaddr_tipc__bindgen_ty_1 {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[test]
fn bindgen_test_layout_sockaddr_tipc() {
    assert_eq!(
        ::std::mem::size_of::<sockaddr_tipc>(),
        16usize,
        concat!("Size of: ", stringify!(sockaddr_tipc))
    );
    assert_eq!(
        ::std::mem::align_of::<sockaddr_tipc>(),
        4usize,
        concat!("Alignment of ", stringify!(sockaddr_tipc))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<sockaddr_tipc>())).family as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc),
            "::",
            stringify!(family)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<sockaddr_tipc>())).addrtype as *const _ as usize },
        2usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc),
            "::",
            stringify!(addrtype)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<sockaddr_tipc>())).scope as *const _ as usize },
        3usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc),
            "::",
            stringify!(scope)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<sockaddr_tipc>())).addr as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(sockaddr_tipc),
            "::",
            stringify!(addr)
        )
    );
}
impl Default for sockaddr_tipc {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub struct tipc_group_req {
    pub type_: __u32,
    pub instance: __u32,
    pub scope: __u32,
    pub flags: __u32,
}
#[test]
fn bindgen_test_layout_tipc_group_req() {
    assert_eq!(
        ::std::mem::size_of::<tipc_group_req>(),
        16usize,
        concat!("Size of: ", stringify!(tipc_group_req))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_group_req>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_group_req))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_group_req>())).type_ as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_group_req),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_group_req>())).instance as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_group_req),
            "::",
            stringify!(instance)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_group_req>())).scope as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_group_req),
            "::",
            stringify!(scope)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_group_req>())).flags as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_group_req),
            "::",
            stringify!(flags)
        )
    );
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct tipc_sioc_ln_req {
    pub peer: __u32,
    pub bearer_id: __u32,
    pub linkname: [::std::os::raw::c_char; 60usize],
}
#[test]
fn bindgen_test_layout_tipc_sioc_ln_req() {
    assert_eq!(
        ::std::mem::size_of::<tipc_sioc_ln_req>(),
        68usize,
        concat!("Size of: ", stringify!(tipc_sioc_ln_req))
    );
    assert_eq!(
        ::std::mem::align_of::<tipc_sioc_ln_req>(),
        4usize,
        concat!("Alignment of ", stringify!(tipc_sioc_ln_req))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_sioc_ln_req>())).peer as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_sioc_ln_req),
            "::",
            stringify!(peer)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_sioc_ln_req>())).bearer_id as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_sioc_ln_req),
            "::",
            stringify!(bearer_id)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<tipc_sioc_ln_req>())).linkname as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(tipc_sioc_ln_req),
            "::",
            stringify!(linkname)
        )
    );
}
impl Default for tipc_sioc_ln_req {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
