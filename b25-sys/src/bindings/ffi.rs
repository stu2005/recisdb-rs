use std::collections::HashSet;
use std::marker::PhantomPinned;
use std::ptr::null_mut;

use cryptography_b25_00::expand_00;
use once_cell::sync::OnceCell;
use tail_cbc::cipher::KeyIvInit;
use tail_cbc::UnalignedBytesDecryptMut;

use crate::access_control::types::Block00CbcDec;
use crate::bindings::arib_std_b25::{
    wchar_t, B_CAS_CARD, B_CAS_CARD_PRIVATE_DATA, B_CAS_ECM_RESULT, B_CAS_ID, B_CAS_INIT_STATUS,
    B_CAS_PWR_ON_CTRL, B_CAS_PWR_ON_CTRL_INFO,
};
use crate::bindings::error::BCasCardError;
use crate::WorkingKey;

pub static KEYS: OnceCell<HashSet<u8, WorkingKey>> = OnceCell::new();

// Overrides the functions of the struct `B_CAS_CARD`

#[no_mangle]
unsafe extern "C" fn release(bcas: *mut ::std::os::raw::c_void) {
    //free private data manually
    Box::from_raw((*(bcas as *mut B_CAS_CARD)).private_data as *mut B_CAS_CARD_PRIVATE_DATA);
}

const DEFAULT_NAME: &str = "b25-sys";
impl Default for B_CAS_CARD_PRIVATE_DATA {
    fn default() -> Self {
        B_CAS_CARD_PRIVATE_DATA {
            mng: 0,
            card: 0,
            pool: null_mut(),
            reader: DEFAULT_NAME.as_ptr() as *const wchar_t,
            sbuf: null_mut(),
            rbuf: null_mut(),
            stat: B_CAS_INIT_STATUS {
                system_key: [
                    0x36, 0x31, 0x04, 0x66, 0x4b, 0x17, 0xea, 0x5c, 0x32, 0xdf, 0x9c, 0xf5, 0xc4,
                    0xc3, 0x6c, 0x1b, 0xec, 0x99, 0x39, 0x21, 0x68, 0x9d, 0x4b, 0xb7, 0xb7, 0x4e,
                    0x40, 0x84, 0x0d, 0x2e, 0x7d, 0x98,
                ],
                init_cbc: [0xfe, 0x27, 0x19, 0x99, 0x19, 0x69, 0x09, 0x11],
                bcas_card_id: 0xfe2719991969091,
                card_status: 0,
                ca_system_id: 5,
            },
            id: B_CAS_ID {
                data: &mut [0i64; 1] as *mut _,
                count: 1,
            },
            id_max: 0,
            pwc: B_CAS_PWR_ON_CTRL_INFO {
                data: &mut B_CAS_PWR_ON_CTRL {
                    s_yy: 0,
                    s_mm: 0,
                    s_dd: 0,
                    l_yy: 0,
                    l_mm: 0,
                    l_dd: 0,
                    hold_time: 0,
                    broadcaster_group_id: 0,
                    network_id: 0,
                    transport_id: 0,
                },
                count: 0,
            },
            pwc_max: 0,
        }
    }
}
#[no_mangle]
unsafe extern "C" fn init(bcas: *mut ::std::os::raw::c_void) -> ::std::os::raw::c_int {
    (*(bcas as *mut B_CAS_CARD)).private_data =
        Box::into_raw(Box::new(B_CAS_CARD_PRIVATE_DATA::default())) as *mut _;
    0
}

#[no_mangle]
unsafe extern "C" fn get_init_status(
    bcas: *mut ::std::os::raw::c_void,
    stat: *mut B_CAS_INIT_STATUS,
) -> ::std::os::raw::c_int {
    let prv = (*(bcas as *mut B_CAS_CARD)).private_data as *mut B_CAS_CARD_PRIVATE_DATA;
    *stat = (*prv).stat; //Copy the status
    0
}

#[no_mangle]
unsafe extern "C" fn proc_ecm(
    bcas: *mut ::std::os::raw::c_void,
    dst: *mut B_CAS_ECM_RESULT,
    src: *mut u8,
    len: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    let mut payload = {
        let recv = &*std::ptr::slice_from_raw_parts(src, len as usize);
        recv.to_vec()
    };

    let ks = {
        let size = payload.len();
        if size < 19 {
            Err(())
        } else {
            let protocol = payload[0];
            let working_key_id = payload[2];
            let cipher = &mut payload[3..size - 1];
            // let k = if working_key_id { &expand_00(0x15f8c5bf840b6694u64, 0) };
            let k = expand_00(0x15f8c5bf840b6694u64, 0);
            let mut dec =
                Block00CbcDec::new(&k, &0xfe27199919690911u64.swap_bytes().to_ne_bytes().into());
            let plaintext = dec.decrypt_bytes_mut(cipher).expect("decryption failed");
            Ok((Vec::from(plaintext), working_key_id))
        }
    };

    if let Ok(result) = ks {
        std::ptr::copy_nonoverlapping(
            result.0.as_ptr(),
            (*dst).scramble_key.as_mut_ptr(),
            result.0.len(),
        );
    }
    (*dst).return_code = 0x0800;
    return 0;
}

#[no_mangle]
unsafe extern "C" fn get_id(
    bcas: *mut ::std::os::raw::c_void,
    dst: *mut B_CAS_ID,
) -> ::std::os::raw::c_int {
    let prv = (*(bcas as *mut B_CAS_CARD)).private_data as *mut B_CAS_CARD_PRIVATE_DATA;
    (*dst).data = (*prv).id.data;
    (*dst).count = (*prv).id.count;
    0
}

unsafe extern "C" fn get_pwr_on_ctrl(
    bcas: *mut ::std::os::raw::c_void,
    dst: *mut B_CAS_PWR_ON_CTRL_INFO,
) -> ::std::os::raw::c_int {
    let prv = (*(bcas as *mut B_CAS_CARD)).private_data as *mut B_CAS_CARD_PRIVATE_DATA;
    (*dst).data = (*prv).pwc.data;
    (*dst).count = (*prv).pwc.count;
    0
}

#[no_mangle]
unsafe extern "C" fn proc_emm(
    bcas: *mut ::std::os::raw::c_void,
    src: *mut u8,
    len: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    0
}

impl Default for B_CAS_CARD {
    fn default() -> Self {
        B_CAS_CARD {
            private_data: null_mut(),
            release: Some(release),
            init: Some(init),
            get_init_status: Some(get_init_status),
            get_id: Some(get_id),
            get_pwr_on_ctrl: Some(get_pwr_on_ctrl),
            proc_ecm: Some(proc_ecm),
            proc_emm: Some(proc_emm),
            _pinned: PhantomPinned,
        }
    }
}

impl B_CAS_CARD {
    pub fn initialize(&mut self) {
        let init = self.init;
        let errno =
            unsafe { init.unwrap()(self as *mut B_CAS_CARD as *mut ::std::os::raw::c_void) };

        if errno != 0 {
            let err = BCasCardError::from(errno);
            panic!("BCasCardError: {}", err);
        }
    }
}