//! Windows Firewall and network-profile checks.
//!
//! Phones reach the companion (and iPhones the AirPlay receiver) only when
//! Windows lets inbound connections through. The two usual blockers are a
//! Wi-Fi marked as a *Public* network (Windows 11's default for a new
//! network) and the first-run firewall prompt being dismissed or answered
//! for the other network type, which leaves *block* rules for the program.
//! Block rules win over allow rules, so the fix removes them.
//!
//! Reading the rules and network categories needs no administrator rights.
//! Changing them runs the elevated helper (`--omnihub-helper firewall-allow`
//! and `network-private`, see [`crate::helper`]).

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const PROFILE_DOMAIN: i32 = 1;
pub const PROFILE_PRIVATE: i32 = 2;
pub const PROFILE_PUBLIC: i32 = 4;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NetworkCategory {
    Public,
    Private,
    Domain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub category: NetworkCategory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    /// An enabled allow rule covers the active network type.
    Allowed,
    /// A block rule (or "block all incoming connections") stops it.
    Blocked,
    /// No rule: Windows blocks it by default.
    NoRule,
    /// The firewall is off for the active network type.
    Off,
    /// Not Windows, or the firewall could not be read.
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Proto {
    Tcp,
    Udp,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    pub name: String,
    pub allow: bool,
    pub enabled: bool,
    pub profiles: i32,
    pub protocol: String,
    pub ports: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirewallReport {
    pub supported: bool,
    pub program: String,
    pub networks: Vec<NetworkInfo>,
    /// Bitmask of the firewall profiles in use (1 domain, 2 private, 4 public).
    pub active_profiles: i32,
    pub verdict: Verdict,
    pub rules: Vec<RuleInfo>,
    pub message: String,
}

impl FirewallReport {
    pub fn unsupported(program: &Path) -> Self {
        FirewallReport {
            supported: false,
            program: program.to_string_lossy().into_owned(),
            networks: vec![],
            active_profiles: 0,
            verdict: Verdict::Unknown,
            rules: vec![],
            message: "Firewall checks are only available on Windows.".into(),
        }
    }
}

/// One rule as the evaluation needs it (independent of COM, so it can be tested).
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub program: String,
    pub inbound: bool,
    pub allow: bool,
    pub enabled: bool,
    pub profiles: i32,
    /// IANA protocol number, 256 = any.
    pub protocol: i32,
    pub local_ports: String,
}

/// Firewall state of one profile.
#[derive(Debug, Clone, Copy)]
pub struct ProfileState {
    pub profile: i32,
    pub enabled: bool,
    pub block_all_inbound: bool,
}

fn norm_path(p: &str) -> String {
    let mut s = p.trim().trim_matches('"').to_string();
    // Rules may store %VARIABLE% paths.
    while let (Some(a), true) = (s.find('%'), s.matches('%').count() >= 2) {
        let Some(b) = s[a + 1..].find('%').map(|b| a + 1 + b) else { break };
        let var = &s[a + 1..b];
        match std::env::var(var) {
            Ok(v) => s.replace_range(a..=b, &v),
            Err(_) => break,
        }
    }
    s.replace('/', "\\").to_lowercase()
}

/// Whether a firewall port list ("*", "47800", "7000-7100,8080") covers `port`.
pub fn ports_cover(list: &str, port: Option<u16>) -> bool {
    let list = list.trim();
    if list.is_empty() || list == "*" || list.eq_ignore_ascii_case("any") {
        return true;
    }
    let Some(port) = port else { return false };
    list.split(',').any(|part| {
        let part = part.trim();
        match part.split_once('-') {
            Some((a, b)) => matches!((a.trim().parse::<u16>(), b.trim().parse::<u16>()), (Ok(a), Ok(b)) if a <= port && port <= b),
            None => part.parse::<u16>() == Ok(port),
        }
    })
}

fn proto_covers(rule: i32, want: Proto) -> bool {
    rule == 256
        || match want {
            Proto::Tcp => rule == 6,
            Proto::Udp => rule == 17,
            Proto::Any => rule == 6 || rule == 17,
        }
}

/// Decide whether inbound traffic to `program` gets through on the active
/// profiles, following Windows Firewall precedence (block beats allow).
pub fn evaluate(rules: &[Rule], profiles: &[ProfileState], program: &str, port: Option<u16>, proto: Proto) -> (Verdict, Vec<RuleInfo>) {
    let want = norm_path(program);
    let relevant: Vec<&Rule> = rules
        .iter()
        .filter(|r| r.inbound && proto_covers(r.protocol, proto))
        .filter(|r| {
            let rp = norm_path(&r.program);
            // Program rules, or port rules that explicitly name our port.
            (rp == want && ports_cover(&r.local_ports, port)) || (r.program.trim().is_empty() && !r.local_ports.trim().is_empty() && r.local_ports.trim() != "*" && ports_cover(&r.local_ports, port))
        })
        .collect();
    let infos = relevant
        .iter()
        .map(|r| RuleInfo {
            name: r.name.clone(),
            allow: r.allow,
            enabled: r.enabled,
            profiles: r.profiles,
            protocol: match r.protocol {
                6 => "TCP".into(),
                17 => "UDP".into(),
                256 => "Any".into(),
                n => n.to_string(),
            },
            ports: if r.local_ports.is_empty() { "*".into() } else { r.local_ports.clone() },
        })
        .collect();
    if profiles.is_empty() {
        return (Verdict::Unknown, infos);
    }
    let mut worst = Verdict::Off;
    for p in profiles {
        let v = if !p.enabled {
            Verdict::Off
        } else if p.block_all_inbound || relevant.iter().any(|r| r.enabled && !r.allow && r.profiles & p.profile != 0) {
            Verdict::Blocked
        } else if relevant.iter().any(|r| r.enabled && r.allow && r.profiles & p.profile != 0) {
            Verdict::Allowed
        } else {
            Verdict::NoRule
        };
        let rank = |v: Verdict| match v {
            Verdict::Off => 0,
            Verdict::Allowed => 1,
            Verdict::Unknown => 2,
            Verdict::NoRule => 3,
            Verdict::Blocked => 4,
        };
        if rank(v) > rank(worst) {
            worst = v;
        }
    }
    (worst, infos)
}

pub fn profile_names(mask: i32) -> Vec<&'static str> {
    let mut v = vec![];
    if mask & PROFILE_DOMAIN != 0 {
        v.push("domain");
    }
    if mask & PROFILE_PRIVATE != 0 {
        v.push("private");
    }
    if mask & PROFILE_PUBLIC != 0 {
        v.push("public");
    }
    v
}

#[cfg_attr(not(windows), allow(dead_code))]
fn message(verdict: Verdict, networks: &[NetworkInfo], active: i32, what: &str) -> String {
    let public = active & PROFILE_PUBLIC != 0 || networks.iter().any(|n| n.category == NetworkCategory::Public);
    match verdict {
        Verdict::Allowed => format!("Windows Firewall lets phones reach {what}."),
        Verdict::Off => "Windows Firewall is off for this network.".into(),
        Verdict::Blocked if public => format!("Windows Firewall blocks {what} on this network, which Windows treats as Public."),
        Verdict::Blocked => format!("A Windows Firewall rule blocks {what}."),
        Verdict::NoRule if public => format!("This network is marked Public, and Windows Firewall has no rule allowing {what} on it."),
        Verdict::NoRule => format!("Windows Firewall has no rule allowing {what}, so it blocks phones by default."),
        Verdict::Unknown => "Could not read the Windows Firewall settings.".into(),
    }
}

/// Check whether inbound connections to `program` get through.
pub fn check(program: &Path, port: Option<u16>, proto: Proto, what: &str) -> FirewallReport {
    #[cfg(windows)]
    {
        let program_s = program.to_string_lossy().into_owned();
        let what = what.to_string();
        let res = std::thread::spawn(move || win::read_state()).join().unwrap_or_else(|_| Err(std::io::Error::other("firewall check panicked")));
        match res {
            Ok((rules, profiles, active, networks)) => {
                let (verdict, infos) = evaluate(&rules, &profiles, &program_s, port, proto);
                FirewallReport { supported: true, message: message(verdict, &networks, active, &what), program: program_s, networks, active_profiles: active, verdict, rules: infos }
            }
            Err(e) => FirewallReport { supported: true, program: program_s, networks: vec![], active_profiles: 0, verdict: Verdict::Unknown, rules: vec![], message: format!("Could not read the Windows Firewall settings: {e}") },
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (port, proto, what);
        FirewallReport::unsupported(program)
    }
}

/// Programs the elevated helper may add firewall rules for: OmniHub itself
/// and an UxPlay AirPlay receiver.
pub fn helper_may_allow(program: &Path) -> bool {
    if !program.is_absolute() || !program.is_file() {
        return false;
    }
    if std::fs::symlink_metadata(program).map(|m| m.file_type().is_symlink()).unwrap_or(true) {
        return false;
    }
    let name = program.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if name == "uxplay.exe" {
        return true;
    }
    match (std::env::current_exe().and_then(|p| p.canonicalize()), program.canonicalize()) {
        (Ok(me), Ok(p)) => me == p,
        _ => false,
    }
}

pub fn is_guid(s: &str) -> bool {
    let s = s.trim_matches(|c| c == '{' || c == '}');
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5 && parts.iter().zip([8, 4, 4, 4, 12]).all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Ask for administrator approval and allow `program` through the firewall
/// on the profiles in `mask`, removing block rules for it first.
pub fn allow_program(program: &Path, rule_name: &str, mask: i32) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let args = vec![crate::helper::HELPER_FLAG.to_string(), "firewall-allow".into(), program.to_string_lossy().into_owned(), rule_name.to_string(), mask.to_string()];
    match crate::system::elevation::run_elevated_and_wait(&exe, &args, false)? {
        0 => Ok(()),
        code => Err(std::io::Error::other(format!("the firewall change failed (code {code})"))),
    }
}

/// Ask for administrator approval and mark a network as Private.
pub fn make_network_private(id: &str) -> std::io::Result<()> {
    if !is_guid(id) {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a network id"));
    }
    let exe = std::env::current_exe()?;
    let args = vec![crate::helper::HELPER_FLAG.to_string(), "network-private".into(), id.to_string()];
    match crate::system::elevation::run_elevated_and_wait(&exe, &args, false)? {
        0 => Ok(()),
        code => Err(std::io::Error::other(format!("changing the network type failed (code {code})"))),
    }
}

/// Helper job: remove inbound block rules for `program`, (re)create our allow rule.
pub fn helper_allow(program: &Path, rule_name: &str, mask: i32) -> i32 {
    if !helper_may_allow(program) || rule_name.is_empty() || rule_name.len() > 100 || mask & !(PROFILE_DOMAIN | PROFILE_PRIVATE | PROFILE_PUBLIC) != 0 || mask == 0 {
        return 64;
    }
    #[cfg(windows)]
    {
        let program = program.to_path_buf();
        let rule_name = rule_name.to_string();
        match std::thread::spawn(move || win::allow(&program, &rule_name, mask)).join() {
            Ok(Ok(())) => 0,
            _ => 1,
        }
    }
    #[cfg(not(windows))]
    {
        1
    }
}

/// Helper job: mark a connected network as Private.
pub fn helper_make_private(id: &str) -> i32 {
    if !is_guid(id) {
        return 64;
    }
    #[cfg(windows)]
    {
        let id = id.to_string();
        match std::thread::spawn(move || win::set_private(&id)).join() {
            Ok(Ok(())) => 0,
            _ => 1,
        }
    }
    #[cfg(not(windows))]
    {
        1
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use windows::core::{Interface, BSTR, GUID};
    use windows::Win32::Foundation::VARIANT_TRUE;
    use windows::Win32::NetworkManagement::WindowsFirewall::*;
    use windows::Win32::Networking::NetworkListManager::*;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::System::Ole::IEnumVARIANT;
    use windows::Win32::System::Variant::{VariantClear, VARIANT, VT_DISPATCH};

    struct Com;
    impl Com {
        fn init() -> Com {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            Com
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    fn io(e: windows::core::Error) -> std::io::Error {
        std::io::Error::other(e.message())
    }

    fn policy() -> std::io::Result<INetFwPolicy2> {
        unsafe { CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER).map_err(io) }
    }

    fn rules(policy: &INetFwPolicy2) -> std::io::Result<Vec<(INetFwRule, Rule)>> {
        let mut out = Vec::new();
        unsafe {
            let all = policy.Rules().map_err(io)?;
            let en: IEnumVARIANT = all._NewEnum().map_err(io)?.cast().map_err(io)?;
            loop {
                let mut v = [VARIANT::default()];
                let mut fetched = 0u32;
                if en.Next(&mut v, &mut fetched).is_err() || fetched == 0 {
                    break;
                }
                let inner = &v[0].Anonymous.Anonymous;
                if inner.vt == VT_DISPATCH {
                    if let Some(d) = inner.Anonymous.pdispVal.as_ref() {
                        if let Ok(r) = d.cast::<INetFwRule>() {
                            let rule = Rule {
                                name: r.Name().map(|s| s.to_string()).unwrap_or_default(),
                                program: r.ApplicationName().map(|s| s.to_string()).unwrap_or_default(),
                                inbound: r.Direction().map(|d| d == NET_FW_RULE_DIR_IN).unwrap_or(false),
                                allow: r.Action().map(|a| a == NET_FW_ACTION_ALLOW).unwrap_or(false),
                                enabled: r.Enabled().map(|e| e.as_bool()).unwrap_or(false),
                                profiles: r.Profiles().unwrap_or(0),
                                protocol: r.Protocol().unwrap_or(256),
                                local_ports: r.LocalPorts().map(|s| s.to_string()).unwrap_or_default(),
                            };
                            out.push((r, rule));
                        }
                    }
                }
                let _ = VariantClear(&mut v[0]);
            }
        }
        Ok(out)
    }

    fn networks() -> Vec<NetworkInfo> {
        let mut out = Vec::new();
        unsafe {
            let nlm: INetworkListManager = match CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL) {
                Ok(n) => n,
                Err(_) => return out,
            };
            let Ok(en) = nlm.GetNetworks(NLM_ENUM_NETWORK_CONNECTED) else { return out };
            loop {
                let mut item = [None];
                let mut fetched = 0u32;
                if en.Next(&mut item, Some(&mut fetched as *mut u32)).is_err() || fetched == 0 {
                    break;
                }
                let Some(n) = item[0].take() else { break };
                let category = match n.GetCategory() {
                    Ok(c) if c == NLM_NETWORK_CATEGORY_PRIVATE => NetworkCategory::Private,
                    Ok(c) if c == NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED => NetworkCategory::Domain,
                    _ => NetworkCategory::Public,
                };
                let id = n.GetNetworkId().map(|g| format!("{g:?}")).unwrap_or_default();
                out.push(NetworkInfo { id, name: n.GetName().map(|s| s.to_string()).unwrap_or_default(), category });
            }
        }
        out
    }

    pub type State = (Vec<Rule>, Vec<ProfileState>, i32, Vec<NetworkInfo>);

    pub fn read_state() -> std::io::Result<State> {
        let _com = Com::init();
        let policy = policy()?;
        let active = unsafe { policy.CurrentProfileTypes().map_err(io)? };
        let mut profiles = Vec::new();
        for p in [PROFILE_DOMAIN, PROFILE_PRIVATE, PROFILE_PUBLIC] {
            if active & p != 0 {
                let t = NET_FW_PROFILE_TYPE2(p);
                unsafe {
                    profiles.push(ProfileState {
                        profile: p,
                        enabled: policy.get_FirewallEnabled(t).map(|b| b.as_bool()).unwrap_or(true),
                        block_all_inbound: policy.get_BlockAllInboundTraffic(t).map(|b| b.as_bool()).unwrap_or(false),
                    });
                }
            }
        }
        let rules = rules(&policy)?.into_iter().map(|(_, r)| r).collect();
        Ok((rules, profiles, active, networks()))
    }

    pub fn allow(program: &Path, rule_name: &str, mask: i32) -> std::io::Result<()> {
        let _com = Com::init();
        let policy = policy()?;
        let want = norm_path(&program.to_string_lossy());
        unsafe {
            let all = policy.Rules().map_err(io)?;
            // Block rules for this program (left by a dismissed prompt) and
            // our own earlier rules are removed. Remove() deletes every rule
            // with that name, so rename the block rules to something unique first.
            for (com, r) in rules(&policy)? {
                if !r.inbound || norm_path(&r.program) != want {
                    continue;
                }
                if !r.allow || r.name == rule_name {
                    let unique = BSTR::from(format!("omnihub-remove-{}", uuid::Uuid::new_v4()));
                    com.SetName(&unique).map_err(io)?;
                    all.Remove(&unique).map_err(io)?;
                }
            }
            let rule: INetFwRule = CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER).map_err(io)?;
            rule.SetName(&BSTR::from(rule_name)).map_err(io)?;
            rule.SetDescription(&BSTR::from("Added by OmniHub so phones on your network can connect.")).map_err(io)?;
            rule.SetApplicationName(&BSTR::from(program.to_string_lossy().as_ref())).map_err(io)?;
            rule.SetProtocol(NET_FW_IP_PROTOCOL_ANY.0).map_err(io)?;
            rule.SetDirection(NET_FW_RULE_DIR_IN).map_err(io)?;
            rule.SetAction(NET_FW_ACTION_ALLOW).map_err(io)?;
            rule.SetProfiles(mask).map_err(io)?;
            rule.SetEnabled(VARIANT_TRUE).map_err(io)?;
            all.Add(&rule).map_err(io)?;
        }
        Ok(())
    }

    pub fn set_private(id: &str) -> std::io::Result<()> {
        let _com = Com::init();
        let guid = GUID::try_from(id.trim_matches(|c| c == '{' || c == '}')).map_err(|_| std::io::Error::other("bad network id"))?;
        unsafe {
            let nlm: INetworkListManager = CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).map_err(io)?;
            let n = nlm.GetNetwork(guid).map_err(io)?;
            n.SetCategory(NLM_NETWORK_CATEGORY_PRIVATE).map_err(io)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(program: &str, allow: bool, profiles: i32) -> Rule {
        Rule { name: "r".into(), program: program.into(), inbound: true, allow, enabled: true, profiles, protocol: 256, local_ports: String::new() }
    }

    const EXE: &str = r"C:\Users\me\AppData\Local\OmniHub\omnihub.exe";

    #[test]
    fn port_lists() {
        assert!(ports_cover("", Some(1)));
        assert!(ports_cover("*", None));
        assert!(ports_cover("47800", Some(47800)));
        assert!(ports_cover("7000-7100, 47800", Some(7050)));
        assert!(!ports_cover("7000-7100", Some(47800)));
        assert!(!ports_cover("47800", None));
    }

    #[test]
    fn block_rules_win_and_public_needs_its_own_rule() {
        let public = [ProfileState { profile: PROFILE_PUBLIC, enabled: true, block_all_inbound: false }];
        let private = [ProfileState { profile: PROFILE_PRIVATE, enabled: true, block_all_inbound: false }];
        // What the first-run prompt leaves when only "Private" was ticked.
        let prompt = [rule(EXE, true, PROFILE_PRIVATE), rule(EXE, false, PROFILE_PUBLIC)];
        assert_eq!(evaluate(&prompt, &private, EXE, Some(47800), Proto::Tcp).0, Verdict::Allowed);
        assert_eq!(evaluate(&prompt, &public, EXE, Some(47800), Proto::Tcp).0, Verdict::Blocked);
        assert_eq!(evaluate(&[], &private, EXE, Some(47800), Proto::Tcp).0, Verdict::NoRule);
        // Case and %LOCALAPPDATA%-style differences still match.
        let upper = [rule(&EXE.to_uppercase(), true, PROFILE_PRIVATE | PROFILE_PUBLIC)];
        assert_eq!(evaluate(&upper, &public, EXE, Some(47800), Proto::Tcp).0, Verdict::Allowed);
        let off = [ProfileState { profile: PROFILE_PUBLIC, enabled: false, block_all_inbound: false }];
        assert_eq!(evaluate(&prompt, &off, EXE, Some(47800), Proto::Tcp).0, Verdict::Off);
        let shields_up = [ProfileState { profile: PROFILE_PRIVATE, enabled: true, block_all_inbound: true }];
        assert_eq!(evaluate(&prompt, &shields_up, EXE, Some(47800), Proto::Tcp).0, Verdict::Blocked);
        // A disabled block rule does not count; a UDP-only rule does not cover TCP.
        let mut disabled = rule(EXE, false, PROFILE_PRIVATE);
        disabled.enabled = false;
        let mut udp = rule(EXE, true, PROFILE_PRIVATE);
        udp.protocol = 17;
        assert_eq!(evaluate(&[disabled, udp], &private, EXE, Some(47800), Proto::Tcp).0, Verdict::NoRule);
        // A port rule for our port counts even without a program.
        let mut port_rule = rule("", true, PROFILE_PRIVATE);
        port_rule.local_ports = "47800".into();
        assert_eq!(evaluate(&[port_rule], &private, EXE, Some(47800), Proto::Tcp).0, Verdict::Allowed);
    }

    #[test]
    fn guids_and_helper_inputs() {
        assert!(is_guid("{DCB00C01-570F-4A9B-8D69-199FDBA5723B}"));
        assert!(is_guid("dcb00c01-570f-4a9b-8d69-199fdba5723b"));
        assert!(!is_guid("dcb00c01-570f-4a9b-8d69"));
        assert!(!is_guid("x; del C:\\"));
        assert_eq!(helper_allow(Path::new("relative.exe"), "x", PROFILE_PRIVATE), 64);
        assert_eq!(helper_allow(Path::new("/bin/sh"), "x", PROFILE_PRIVATE), 64);
        assert_eq!(helper_make_private("nope"), 64);
        assert_eq!(profile_names(PROFILE_PRIVATE | PROFILE_PUBLIC), vec!["private", "public"]);
    }

    #[test]
    fn messages_name_the_public_network_problem() {
        let public = [NetworkInfo { id: String::new(), name: "Home".into(), category: NetworkCategory::Public }];
        assert!(message(Verdict::NoRule, &public, PROFILE_PUBLIC, "OmniHub").contains("Public"));
        assert!(message(Verdict::Allowed, &[], PROFILE_PRIVATE, "OmniHub").contains("lets phones"));
    }
}
