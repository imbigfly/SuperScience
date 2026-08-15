$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    # Use the desktop master, not ui/logo.svg. The in-app mark bakes a rounded
    # clip and fills the canvas; macOS then masks again and the Dock icon looks
    # oversized compared with Linux/Docker. See icons/app-icon.svg.
    $logo = Resolve-Path "icons/app-icon.svg"
    $icon = Join-Path $PSScriptRoot "icons/icon.ico"
    if ((Test-Path $icon) -and ((Get-Item $logo).LastWriteTimeUtc -le (Get-Item $icon).LastWriteTimeUtc)) {
        return
    }
    cargo tauri icon $logo
} finally {
    Pop-Location
}
