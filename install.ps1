# Installer for Engram Sentinel (Windows)
$ErrorActionPreference = "Stop"

Write-Host "🔍 Downloading Engram Sentinel..."

# Latest stable binary URL
$binaryUrl = "https://github.com/EngramAI-io/Core/releases/download/v0.1.0/engram-sentinel-windows-x86_64.exe"

# Install directory
$targetDir = "$env:LOCALAPPDATA\EngramAI"
if (!(Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir | Out-Null
}

$binaryPath = Join-Path $targetDir "sentinel.exe"

# Download binary
Invoke-WebRequest -Uri $binaryUrl -OutFile $binaryPath

Write-Host "✅ Download complete: $binaryPath"
Write-Host "🚀 Adding to PATH..."

$oldPath = [Environment]::GetEnvironmentVariable("Path", "User")

if ($oldPath -notlike "*$targetDir*") {
    $newPath = "$oldPath;$targetDir"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "✨ Sentinel added to PATH."
} else {
    Write-Host "ℹ️ PATH already contains Sentinel directory."
}

Write-Host ""
Write-Host "🎉 Installation complete!"
Write-Host "Run Sentinel using:"
Write-Host ""
Write-Host "    sentinel run -- npx -y @modelcontextprotocol/server-filesystem"
Write-Host ""
