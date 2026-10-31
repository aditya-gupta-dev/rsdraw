//! Native Excalidraw-compatible geometry, text layout, and Cairo drawing.
//! Independent of GTK, Wayland, application state, and input delivery.
pub mod canvas;
pub mod fonts;
pub mod freehand;
pub mod rendering;
mod rough;
pub mod text;
mod variable_freehand;
