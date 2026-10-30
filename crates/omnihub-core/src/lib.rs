//! OmniHub core library.
//!
//! Everything that does not depend on the desktop shell lives here so it can
//! be unit tested on any OS, reused by the headless companion server, and
//! wrapped by the Tauri app.

pub mod apps;
pub mod audit;
pub mod browser;
pub mod capture;
pub mod chats;
pub mod claude;
pub mod companion;
pub mod core;
pub mod crashlog;
pub mod db;
pub mod events;
pub mod games;
pub mod govee;
pub mod helper;
pub mod mcserver;
pub mod media;
pub mod mods;
pub mod notes;
pub mod play;
pub mod paths;
pub mod remote;
pub mod rig;
pub mod settings;
pub mod startup;
pub mod storage;
pub mod system;
pub mod thumbs;
pub mod update;
pub mod vault;
pub mod winget;
