use libc::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_ushort, c_void};
use std::ffi::CStr;

extern "C" {
    #[no_mangle]
    fn snprintf(_: *mut c_char, _: c_ulong, _: *const c_char, _: ...) -> c_int;
    #[no_mangle]
    fn strncpy(_: *mut c_char, _: *const c_char, _: c_ulong) -> *mut c_char;
    #[no_mangle]
    fn strlen(_: *const c_char) -> c_ulong;
    #[no_mangle]
    fn shmctl(__shmid: c_int, __cmd: c_int, __buf: *mut shmid_ds) -> c_int;
    #[no_mangle]
    fn shmget(__key: c_int, __size: c_ulong, __shmflg: c_int) -> c_int;
    #[no_mangle]
    fn shmat(__shmid: c_int, __shmaddr: *const c_void, __shmflg: c_int) -> *mut c_void;
    #[no_mangle]
    fn strtol(_: *const c_char, _: *mut *mut c_char, _: c_int) -> c_long;
    #[no_mangle]
    fn setenv(__name: *const c_char, __value: *const c_char, __replace: c_int) -> c_int;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ipc_perm {
    pub __key: c_int,
    pub uid: c_uint,
    pub gid: c_uint,
    pub cuid: c_uint,
    pub cgid: c_uint,
    pub mode: c_ushort,
    pub __pad1: c_ushort,
    pub __seq: c_ushort,
    pub __pad2: c_ushort,
    pub __glibc_reserved1: c_ulong,
    pub __glibc_reserved2: c_ulong,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct shmid_ds {
    pub shm_perm: ipc_perm,
    pub shm_segsz: c_ulong,
    pub shm_atime: c_long,
    pub shm_dtime: c_long,
    pub shm_ctime: c_long,
    pub shm_cpid: c_int,
    pub shm_lpid: c_int,
    pub shm_nattch: c_ulong,
    pub __glibc_reserved4: c_ulong,
    pub __glibc_reserved5: c_ulong,
}
pub const AFL_RET_EMPTY: c_uint = 20;
pub const AFL_RET_ERROR_INPUT_COPY: c_uint = 19;
pub const AFL_RET_TRIM_FAIL: c_uint = 18;
pub const AFL_RET_NO_FUZZ_WORKERS: c_uint = 17;
pub const AFL_RET_ERROR_INITIALIZE: c_uint = 16;
pub const AFL_RET_QUEUE_ENDS: c_uint = 15;
pub const AFL_RET_WRITE_TO_CRASH: c_uint = 14;
pub const AFL_RET_NULL_QUEUE_ENTRY: c_uint = 13;
pub const AFL_RET_ERRNO: c_uint = 12;
pub const AFL_RET_NULL_PTR: c_uint = 11;
pub const AFL_RET_BROKEN_TARGET: c_uint = 10;
pub const AFL_RET_EXEC_ERROR: c_uint = 9;
pub const AFL_RET_ARRAY_END: c_uint = 8;
pub const AFL_RET_SHORT_WRITE: c_uint = 7;
pub const AFL_RET_SHORT_READ: c_uint = 6;
pub const AFL_RET_FILE_SIZE: c_uint = 5;
pub const AFL_RET_FILE_OPEN_ERROR: c_uint = 4;
pub const AFL_RET_ALLOC: c_uint = 3;
pub const AFL_RET_FILE_DUPLICATE: c_uint = 2;
pub const AFL_RET_UNKNOWN_ERROR: c_uint = 1;
pub const AFL_RET_SUCCESS: c_uint = 0;

// A generic sharememory region to be used by any functions (queues or feedbacks
// too.)

#[derive(Copy, Clone)]
#[repr(C)]
pub struct AflShmem {
    pub shm_str: [u8; 20],
    pub shm_id: c_int,
    pub map: *mut c_uchar,
    pub map_size: c_ulong,
}

pub unsafe fn afl_shmem_deinit(mut shm: *mut AflShmem) {
    if shm.is_null() || (*shm).map.is_null() {
        /* Serialized map id */
        // Not set or not initialized;
        return;
    }
    (*shm).shm_str[0 as usize] = '\u{0}' as u8;
    shmctl((*shm).shm_id, 0 as c_int, 0 as *mut shmid_ds);
    (*shm).map = 0 as *mut c_uchar;
}
// Functions to create Shared memory region, for observation channels and
// opening inputs and stuff.
pub unsafe fn afl_shmem_init(mut shm: *mut AflShmem, map_size: c_ulong) -> *mut c_uchar {
    (*shm).map_size = map_size;
    (*shm).map = 0 as *mut c_uchar;
    (*shm).shm_id = shmget(
        0 as c_int,
        map_size,
        0o1000 as c_int | 0o2000 as c_int | 0o600 as c_int,
    );
    if (*shm).shm_id < 0 as c_int {
        (*shm).shm_str[0] = '\u{0}' as u8;
        return 0 as *mut c_uchar;
    }
    snprintf(
        (*shm).shm_str.as_mut_ptr() as *mut i8,
        ::std::mem::size_of::<[c_char; 20]>() as c_ulong,
        b"%d\x00" as *const u8 as *const c_char,
        (*shm).shm_id,
    );
    (*shm).shm_str[(::std::mem::size_of::<[c_char; 20]>() as c_ulong)
        .wrapping_sub(1 as c_int as c_ulong) as usize] = '\u{0}' as u8;
    (*shm).map = shmat((*shm).shm_id, 0 as *const c_void, 0 as c_int) as *mut c_uchar;
    if (*shm).map == -(1 as c_int) as *mut c_void as *mut c_uchar || (*shm).map.is_null() {
        shmctl((*shm).shm_id, 0 as c_int, 0 as *mut shmid_ds);
        (*shm).shm_id = -(1 as c_int);
        (*shm).shm_str[0 as c_int as usize] = '\u{0}' as u8;
        return 0 as *mut c_uchar;
    }
    return (*shm).map;
}

pub unsafe fn afl_shmem_by_str(
    mut shm: *mut AflShmem,
    shm_str: &CStr,
    map_size: c_ulong,
) -> *mut c_uchar {
    if shm.is_null() || shm_str.to_bytes().len() == 0 || map_size == 0 {
        return 0 as *mut c_uchar;
    }
    (*shm).map = 0 as *mut c_uchar;
    (*shm).map_size = map_size;
    strncpy(
        (*shm).shm_str.as_mut_ptr() as *mut c_char,
        shm_str.as_ptr() as *const c_char,
        (::std::mem::size_of::<[c_char; 20]>() as c_ulong).wrapping_sub(1 as c_int as c_ulong),
    );
    (*shm).shm_id = shm_str
        .to_str()
        .expect(&format!("illegal shm_str {:?}", shm_str))
        .parse::<i32>()
        .unwrap();
    (*shm).map = shmat((*shm).shm_id, 0 as *const c_void, 0 as c_int) as *mut c_uchar;
    if (*shm).map == -(1 as c_int) as *mut c_void as *mut c_uchar {
        (*shm).map = 0 as *mut c_uchar;
        (*shm).map_size = 0 as c_int as c_ulong;
        (*shm).shm_str[0 as c_int as usize] = '\u{0}' as u8;
        return 0 as *mut c_uchar;
    }
    return (*shm).map;
}

/* Write sharedmap as env var */
/* Write sharedmap as env var and the size as name#_SIZE */
pub unsafe fn afl_shmem_to_env_var(shmem: &AflShmem, env_name: &CStr) -> c_uint {
    let env_len = env_name.to_bytes().len();
    if env_len == 0 || env_len > 200 || (*shmem).shm_str[0 as c_int as usize] == 0 {
        return AFL_RET_NULL_PTR;
    }
    let mut shm_str: [c_char; 256] = [0; 256];
    snprintf(
        shm_str.as_mut_ptr(),
        ::std::mem::size_of::<[c_char; 256]>() as c_ulong,
        b"%d\x00" as *const u8 as *const c_char,
        (*shmem).shm_id,
    );
    if setenv(
        env_name.as_ptr() as *const c_char,
        shm_str.as_mut_ptr(),
        1 as c_int,
    ) < 0 as c_int
    {
        return AFL_RET_ERRNO;
    }
    /* Write the size to env, too */
    let mut size_env_name: [c_char; 256] = [0; 256];
    snprintf(
        size_env_name.as_mut_ptr(),
        ::std::mem::size_of::<[c_char; 256]>() as c_ulong,
        b"%s_SIZE\x00" as *const u8 as *const c_char,
        env_name,
    );
    snprintf(
        shm_str.as_mut_ptr(),
        ::std::mem::size_of::<[c_char; 256]>() as c_ulong,
        b"%d\x00" as *const u8 as *const c_char,
        (*shmem).shm_id,
    );
    if setenv(size_env_name.as_mut_ptr(), shm_str.as_mut_ptr(), 1 as c_int) < 0 as c_int {
        return AFL_RET_ERRNO;
    }
    return AFL_RET_SUCCESS;
}