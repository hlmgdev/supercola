//! Clipboard via data-control, com `ext` preferido e `wlr` como reserva.
use std::sync::Arc;

pub mod ext {
    use wayland_protocols::ext::data_control::v1::client::{
        ext_data_control_device_v1::{self as dev, ExtDataControlDeviceV1 as Device},
        ext_data_control_manager_v1::ExtDataControlManagerV1 as Manager,
        ext_data_control_offer_v1::{self as ofr, ExtDataControlOfferV1 as Offer},
        ext_data_control_source_v1::{self as src, ExtDataControlSourceV1 as Source},
    };
    const BACKEND: &str = "ext-data-control";
    include!("dc_impl.rs");
}

pub mod wlr {
    use wayland_protocols_wlr::data_control::v1::client::{
        zwlr_data_control_device_v1::{self as dev, ZwlrDataControlDeviceV1 as Device},
        zwlr_data_control_manager_v1::ZwlrDataControlManagerV1 as Manager,
        zwlr_data_control_offer_v1::{self as ofr, ZwlrDataControlOfferV1 as Offer},
        zwlr_data_control_source_v1::{self as src, ZwlrDataControlSourceV1 as Source},
    };
    const BACKEND: &str = "wlr-data-control";
    include!("dc_impl.rs");
}

pub fn watch() -> crate::R {
    ext::watch().or_else(|e| {
        eprintln!("ext indisponível ({e}); tentando wlr");
        wlr::watch()
    })
}

pub fn serve(text: Arc<String>) -> crate::R {
    ext::serve(text.clone()).or_else(|e| {
        eprintln!("ext indisponível ({e}); tentando wlr");
        wlr::serve(text)
    })
}
