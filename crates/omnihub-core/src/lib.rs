//! OmniHub core library.
//!
//! Everything that does not depend on the desktop shell lives here so it can
//! be unit tested on any OS, reused by the headless companion server, and
//! wrapped by the Tauri app.

pub mod app_updates;
pub mod apps;
pub mod askpc;
pub mod audit;
pub mod autoclick;
pub mod backup;
pub mod browser;
pub mod capri_app;
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
pub mod heat;
pub mod helper;
pub mod karaoke;
pub mod nightlight;
pub mod images;
pub mod mcserver;
pub mod media;
pub mod mods;
pub mod files;
pub mod focus;
pub mod notes;
pub mod obs;
pub mod pads;
pub mod play;
pub mod presence;
pub mod quick;
pub mod quiet;
pub mod party;
pub mod paths;
pub mod pkm;
pub mod remote;
pub mod rig;
pub mod rules;
pub mod scan;
pub mod screentime;
pub mod settings;
pub mod sites;
pub mod startup;
pub mod storage;
pub mod system;
pub mod theme_art;
pub mod thumbs;
pub mod update;
pub mod videotools;
pub mod voicecmd;
pub mod watchers;
pub mod web;
pub mod vault;
pub mod winget;
