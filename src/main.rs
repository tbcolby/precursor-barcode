//! Barcode Generator for Precursor: Code 128, Code 39, EAN-13 and UPC-A on
//! the 1-bit display.
//!
//! The encoders and the saved-code model live in `barcode-core` (host-tested).
//! This crate registers with GAM, routes messages, persists, and draws.

mod app;
mod ui;

use num_traits::{FromPrimitive, ToPrimitive};
use precursor_appkit::{app as kit, keys, store};

/// xous-names server name. Any unique string; NOT the GAM app name.
const SERVER_NAME: &str = "_Barcode Generator_";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub(crate) enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    BasisChanged,
    Quit,
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("barcode PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    // GAM app name comes from apps/manifest.json key "barcode".
    let reg = kit::register(
        &xns,
        sid,
        gam::APP_NAME_BARCODE,
        gam::UxType::Framebuffer,
        kit::Opcodes {
            redraw: AppOp::Redraw.to_u32().unwrap(),
            rawkeys: Some(AppOp::Rawkeys.to_u32().unwrap()),
            focus_change: Some(AppOp::FocusChange.to_u32().unwrap()),
            got_input: None,
        },
    );

    let settings_store = store::Store::open(app::SETTINGS_DICT);
    let codes_store = store::Store::open(app::CODES_DICT);
    store::watch_bases(xous::connect(sid).unwrap(), AppOp::BasisChanged.to_usize().unwrap());

    let mut app = app::App::new(reg.screen, settings_store, codes_store);
    let mut foreground = true;

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                if foreground {
                    app.draw();
                }
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                // Keys are never logged: the user may be typing a credential.
                for key in keys::decode_rawkeys([k1, k2, k3, k4]) {
                    if app.on_key(key) == app::Action::Leave {
                        kit::leave(app.screen());
                    }
                }
                if foreground {
                    app.draw();
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, code, _, _, _, {
                match kit::focus_state(code) {
                    gam::FocusState::Foreground => {
                        foreground = true;
                        app.on_foreground();
                    }
                    gam::FocusState::Background => {
                        foreground = false;
                        app.on_background();
                    }
                }
            }),
            Some(AppOp::BasisChanged) => app.reload(),
            Some(AppOp::Quit) => break,
            None => log::warn!("unknown opcode {:?}", msg.body.id()),
        }
    }
    app.on_background();
    kit::shutdown(&xns, sid)
}
