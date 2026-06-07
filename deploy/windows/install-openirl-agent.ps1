param(
  [string]$BinaryPath = "C:\\Program Files\\OpenIRL\\openirl-agent.exe",
  [string]$ConfigPath = "C:\\ProgramData\\OpenIRL\\openirl.toml"
)

New-Service -Name "OpenIRLAgent" `
  -BinaryPathName "`"$BinaryPath`" serve --config `"$ConfigPath`"" `
  -DisplayName "OpenIRL Agent" `
  -StartupType Automatic

Write-Host "Installed OpenIRLAgent service."
