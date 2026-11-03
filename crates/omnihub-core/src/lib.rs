//! OmniHub core library.
//!
//! Everything that does not depend on the desktop shell lives here so it can
//! be unit tested on any OS, reused by the headless companion server, and
//! wrapped by the Tauri app.

pub mod apps;
pub mod audit;
pub mod autoclick;
pub mod browser;
pub mod capture;
pub mod chats;
pub mod claude;
pub mod companion;
pub mod core;
pub mod counters;
pub mod counters_overlay;
pub mod crashlog;
pub mod db;
pub mod devtools;
pub mod downloads;
pub mod events;
pub mod games;
pub mod govee;
pub mod health;
pub mod helper;
pub mod images;
pub mod mcserver;
pub mod media;
pub mod mods;
pub mod files;
pub mod focus;
pub mod notes;
pub mod play;
pub mod quick;
pub mod paths;
pub mod pkm;
pub mod remote;
pub mod rig;
pub mod rules;
pub mod scan;
pub mod settings;
pub mod startup;
pub mod storage;
pub mod system;
pub mod thumbs;
pub mod update;
pub mod videotools;
pub mod vault;
pub mod winget;
