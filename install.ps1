$ErrorActionPreference = "Stop"

$Repo = "EngramAI-io/Core"
$BinaryName = "sentinel.exe"
$Target = "x86_64-pc-windows-msvc"
$Archive = "sentinel-$Target.zip"

$InstallDir = if ($env:INSTALL_DIR) {
    $env:INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "Programs\Sentinel"
}

$Url = "https://github.com/$Repo/releases/latest/download/$Archive"

Write-Host "Installing Sentinel..."
Write-Host "Download: $Url"
Write-Host "Install dir: $InstallDir"

$temp = New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid())
$zipPath = Join-Path $temp $Archive

Invoke-WebRequest $Url -OutFile $zipPath

Expand-Archive $zipPath -DestinationPath $temp -Force

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $temp $BinaryName) $InstallDir -Force

# Add to PATH (user scope)
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable(
        "Path",
        "$UserPath;$InstallDir",
        "User"
    )
    Write-Host "Added Sentinel to PATH (user scope)"
}

Write-Host ""
Write-Host "Sentinel installed successfully."
Write-Host "Restart PowerShell, then run:"
Write-Host "  sentinel --help"
