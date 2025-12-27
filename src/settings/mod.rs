//! GTK4 Settings Dialog
//!
//! Provides a graphical settings window for configuring OSWispa.

#[cfg(feature = "gui")]
mod dialog;

#[cfg(feature = "gui")]
pub use dialog::*;

#[cfg(not(feature = "gui"))]
pub fn show_settings_dialog(_config: &crate::Config) {
    tracing::warn!("Settings dialog requires the 'gui' feature. Rebuild with: cargo build --features gui");
}
