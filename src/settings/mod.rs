//! GTK4 Settings Dialog
//!
//! Provides a graphical settings window for configuring OSWispa.

#[cfg(feature = "gui")]
mod dialog;

#[cfg(feature = "gui")]
pub use dialog::*;

#[cfg(not(feature = "gui"))]
pub fn show_settings_dialog(
    _config: &std::sync::Arc<std::sync::RwLock<crate::Config>>,
    _event_tx: crossbeam_channel::Sender<crate::AppEvent>,
) {
    tracing::warn!(
        "Settings dialog requires the 'gui' feature. Rebuild with: cargo build --features gui"
    );
}
