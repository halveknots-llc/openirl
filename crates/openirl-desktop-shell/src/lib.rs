//! Desktop shell and tray-menu planning.

use serde::{Deserialize, Serialize};

/// Tray action kind.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrayAction {
    /// Open dashboard URL.
    OpenDashboard,
    /// Start local agent process.
    StartAgent,
    /// Stop local agent process.
    StopAgent,
    /// Show runtime readiness.
    Readiness,
    /// Switch OBS to privacy scene.
    Privacy,
    /// Quit shell.
    Quit,
}

/// Tray menu item.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrayMenuItem {
    /// Stable item ID.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Action invoked by the item.
    pub action: TrayAction,
    /// Whether the item is enabled.
    pub enabled: bool,
}

/// Desktop shell plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DesktopShellPlan {
    /// Dashboard URL opened from tray.
    pub dashboard_url: String,
    /// Agent command used by the shell.
    pub agent_command: String,
    /// Tauri-ready frontend directory.
    pub frontend_dir: String,
    /// Tray menu items.
    pub tray_menu: Vec<TrayMenuItem>,
    /// Packaging notes.
    pub packaging_notes: Vec<String>,
}

/// Builds the default tray shell plan.
#[must_use]
pub fn default_desktop_shell_plan(dashboard_url: impl Into<String>) -> DesktopShellPlan {
    let dashboard_url = dashboard_url.into();
    DesktopShellPlan {
        dashboard_url: dashboard_url.clone(),
        agent_command: "openirl-agent serve --config config/openirl.example.toml".to_string(),
        frontend_dir: "apps/openirl-agent/static".to_string(),
        tray_menu: vec![
            item("open-dashboard", "Open Dashboard", TrayAction::OpenDashboard, true),
            item("readiness", "Check Readiness", TrayAction::Readiness, true),
            item("privacy", "Privacy Scene", TrayAction::Privacy, true),
            item("start-agent", "Start Agent", TrayAction::StartAgent, true),
            item("stop-agent", "Stop Agent", TrayAction::StopAgent, true),
            item("quit", "Quit", TrayAction::Quit, true),
        ],
        packaging_notes: vec![
            "Tauri v2 shell can wrap this plan without changing the Rust agent API.".to_string(),
            "Windows-first releases should package openirl-agent.exe and openirl-desktop.exe together.".to_string(),
            format!("Default dashboard URL: {dashboard_url}"),
        ],
    }
}

fn item(id: &str, label: &str, action: TrayAction, enabled: bool) -> TrayMenuItem {
    TrayMenuItem {
        id: id.to_string(),
        label: label.to_string(),
        action,
        enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_plan_contains_privacy_action() {
        let plan = default_desktop_shell_plan("http://127.0.0.1:7707/");
        assert!(
            plan.tray_menu
                .iter()
                .any(|item| item.action == TrayAction::Privacy)
        );
    }
}
