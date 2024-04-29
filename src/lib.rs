pub mod app;

#[cfg(feature = "ssr")]
pub mod fallback;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;

    // initializes logging using the `log` crate
    //_ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    // there are now distinct functions for hydrating and CSR mounting, as opposed to features
    // changing the behavior
    leptos::mount::hydrate_body(App);
}
