//! Installer and service planning for local-first deployments.

use openirl_config::AppConfig;
use serde::{Deserialize, Serialize};

/// Install target type.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallTarget {
    /// Windows service installed with WiX/MSI or PowerShell service commands.
    WindowsService,
    /// Windows current-user startup entry.
    WindowsUserStartup,
    /// Portable zip with no host registration.
    PortableZip,
    /// Linux systemd user service.
    SystemdUser,
    /// Docker relay host.
    DockerRelay,
}

/// One planned command.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallCommand {
    /// Command label.
    pub label: String,
    /// Shell family.
    pub shell: String,
    /// Command text.
    pub command: String,
    /// Whether elevated privileges are required.
    pub requires_admin: bool,
}

/// Planned file in an install package.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallFile {
    /// Source path within release artifact.
    pub source: String,
    /// Destination path.
    pub destination: String,
    /// Whether this file is generated locally.
    pub generated: bool,
}

/// Install plan request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallPlanRequest {
    /// Target install mode.
    pub target: InstallTarget,
    /// Install directory.
    pub install_dir: String,
    /// Config file path.
    pub config_path: String,
    /// Service name.
    pub service_name: String,
}

impl Default for InstallPlanRequest {
    fn default() -> Self {
        Self {
            target: InstallTarget::WindowsService,
            install_dir: "C:\\Program Files\\OpenIRL".to_string(),
            config_path: "C:\\ProgramData\\OpenIRL\\openirl.toml".to_string(),
            service_name: "OpenIRLAgent".to_string(),
        }
    }
}

/// Install plan response.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallPlan {
    /// Request echoed back.
    pub request: InstallPlanRequest,
    /// Files to include/copy.
    pub files: Vec<InstallFile>,
    /// Commands to run.
    pub commands: Vec<InstallCommand>,
    /// Environment variable reminders.
    pub environment: Vec<String>,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
    /// Post-install checks.
    pub next_steps: Vec<String>,
}

/// Builds the default Windows service plan.
#[must_use]
pub fn default_windows_service_plan(config: &AppConfig) -> InstallPlan {
    build_install_plan(config, &InstallPlanRequest::default())
}

/// Builds an install plan for the requested target.
#[must_use]
pub fn build_install_plan(config: &AppConfig, request: &InstallPlanRequest) -> InstallPlan {
    match request.target {
        InstallTarget::WindowsService => windows_service_plan(config, request),
        InstallTarget::WindowsUserStartup => windows_startup_plan(config, request),
        InstallTarget::PortableZip => portable_zip_plan(config, request),
        InstallTarget::SystemdUser => systemd_user_plan(config, request),
        InstallTarget::DockerRelay => docker_relay_plan(config, request),
    }
}

fn common_files(request: &InstallPlanRequest) -> Vec<InstallFile> {
    vec![
        file(
            "openirl-agent.exe",
            format!("{}\\openirl-agent.exe", request.install_dir),
            false,
        ),
        file(
            "openirl-desktop.exe",
            format!("{}\\openirl-desktop.exe", request.install_dir),
            false,
        ),
        file(
            "config/openirl.example.toml",
            request.config_path.clone(),
            true,
        ),
        file(
            "apps/openirl-agent/static",
            format!("{}\\static", request.install_dir),
            false,
        ),
    ]
}

fn windows_service_plan(config: &AppConfig, request: &InstallPlanRequest) -> InstallPlan {
    let mut warnings = Vec::new();
    if !config.api.bind.ip().is_loopback() {
        warnings.push(
            "API is configured for LAN/public bind; set a dashboard token before service install."
                .to_string(),
        );
    }
    InstallPlan {
        request: request.clone(),
        files: common_files(request),
        commands: vec![
            command("create-service", "powershell", format!(
                "New-Service -Name {} -BinaryPathName '\"{}\\openirl-agent.exe\" serve --config \"{}\"' -StartupType Automatic",
                request.service_name, request.install_dir, request.config_path
            ), true),
            command("start-service", "powershell", format!("Start-Service -Name {}", request.service_name), true),
            command("open-firewall-local-only", "powershell", "Keep Windows Firewall scoped to LocalSubnet or localhost unless using a VPN relay.".to_string(), true),
        ],
        environment: vec![
            config.obs.password_env.clone(),
            config.security.dashboard_token_env.clone(),
            config.relay.passphrase_env.clone(),
        ],
        warnings,
        next_steps: vec![
            "Open http://127.0.0.1:7707/ after the service starts.".to_string(),
            "Run /api/runtime/readiness before using a real stream.".to_string(),
            "Switch obs.adapter to web-socket only after OBS WebSocket password is configured.".to_string(),
        ],
    }
}

fn windows_startup_plan(config: &AppConfig, request: &InstallPlanRequest) -> InstallPlan {
    let mut plan = windows_service_plan(config, request);
    plan.request.target = InstallTarget::WindowsUserStartup;
    plan.commands = vec![command(
        "current-user-startup-shortcut",
        "powershell",
        format!(
            "Create a startup shortcut to {}\\openirl-desktop.exe",
            request.install_dir
        ),
        false,
    )];
    plan
}

fn portable_zip_plan(config: &AppConfig, request: &InstallPlanRequest) -> InstallPlan {
    InstallPlan {
        request: request.clone(),
        files: common_files(request),
        commands: vec![command(
            "run-portable-agent",
            "powershell",
            format!("{}\\openirl-agent.exe serve --config {}", request.install_dir, request.config_path),
            false,
        )],
        environment: vec![config.obs.password_env.clone(), config.security.dashboard_token_env.clone()],
        warnings: vec!["Portable mode does not auto-start after reboot unless the user adds a startup shortcut.".to_string()],
        next_steps: vec!["Run scripts/smoke/e2e-local-direct.ps1 after first launch.".to_string()],
    }
}

fn systemd_user_plan(config: &AppConfig, request: &InstallPlanRequest) -> InstallPlan {
    InstallPlan {
        request: request.clone(),
        files: vec![
            file("openirl-agent", "$HOME/.local/bin/openirl-agent", false),
            file(
                "deploy/systemd/openirl-agent.service",
                "$HOME/.config/systemd/user/openirl-agent.service",
                true,
            ),
        ],
        commands: vec![
            command(
                "reload-systemd-user",
                "sh",
                "systemctl --user daemon-reload".to_string(),
                false,
            ),
            command(
                "enable-agent",
                "sh",
                "systemctl --user enable --now openirl-agent".to_string(),
                false,
            ),
        ],
        environment: vec![
            config.obs.password_env.clone(),
            config.security.dashboard_token_env.clone(),
        ],
        warnings: vec![
            "Linux desktop packaging is secondary until Windows-first smoke tests pass."
                .to_string(),
        ],
        next_steps: vec!["Verify http://127.0.0.1:7707/health.".to_string()],
    }
}

fn docker_relay_plan(config: &AppConfig, request: &InstallPlanRequest) -> InstallPlan {
    InstallPlan {
        request: request.clone(),
        files: vec![
            file("deploy/docker-compose.relay.yml", "./docker-compose.yml", false),
            file("deploy/mediamtx/openirl.mediamtx.yml", "./openirl.mediamtx.yml", false),
        ],
        commands: vec![command("start-relay", "sh", "docker compose up -d".to_string(), false)],
        environment: vec![config.relay.passphrase_env.clone()],
        warnings: vec!["Relay host security, firewalling, and VPN exposure must be reviewed before public use.".to_string()],
        next_steps: vec!["Point ingest.public_host at the relay host and regenerate encoder profiles.".to_string()],
    }
}

fn file(source: impl Into<String>, destination: impl Into<String>, generated: bool) -> InstallFile {
    InstallFile {
        source: source.into(),
        destination: destination.into(),
        generated,
    }
}

fn command(
    label: impl Into<String>,
    shell: impl Into<String>,
    command: impl Into<String>,
    requires_admin: bool,
) -> InstallCommand {
    InstallCommand {
        label: label.into(),
        shell: shell.into(),
        command: command.into(),
        requires_admin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_plan_has_service_command() {
        let plan = default_windows_service_plan(&AppConfig::default());
        assert!(
            plan.commands
                .iter()
                .any(|command| command.label == "create-service")
        );
    }
}
