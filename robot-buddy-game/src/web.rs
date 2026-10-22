//! Browser APIs the game uses, via `web-sys`: localStorage, tab visibility,
//! speech synthesis, file download, and the wall clock. WASM-only; native
//! builds have their own stand-ins at each call site.

use wasm_bindgen::JsCast;

fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

fn storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok().flatten()
}

pub fn storage_get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}

pub fn page_is_hidden() -> bool {
    window()
        .and_then(|w| w.document())
        .is_some_and(|d| d.hidden() || d.visibility_state() == web_sys::VisibilityState::Hidden)
}

/// Speak `text`, cutting off whatever was being said.
pub fn speak(text: &str, pitch: f32, rate: f32) {
    let Some(synth) = window().and_then(|w| w.speech_synthesis().ok()) else { return };
    synth.cancel();
    let Ok(u) = web_sys::SpeechSynthesisUtterance::new_with_text(text) else { return };
    u.set_pitch(pitch);
    u.set_rate(rate);
    u.set_volume(0.8);
    synth.speak(&u);
}

pub fn cancel_speech() {
    if let Some(synth) = window().and_then(|w| w.speech_synthesis().ok()) {
        synth.cancel();
    }
}

/// Offer `data` to the user as a file download named `filename`.
pub fn download(data: &str, filename: &str, mime: &str) {
    let Some(document) = window().and_then(|w| w.document()) else { return };
    let parts = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(data));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };
    if let Some(a) = document.create_element("a").ok().and_then(|e| e.dyn_into::<web_sys::HtmlAnchorElement>().ok()) {
        a.set_href(&url);
        a.set_download(filename);
        a.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}

/// Seconds since the Unix epoch.
pub fn now_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// Today's date as `YYYY-MM-DDTHH:MM:SS.sssZ`.
pub fn iso_date() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}
