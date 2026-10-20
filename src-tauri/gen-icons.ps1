$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    # One three-wisp mark, with platform-specific tile geometry. Legacy ICNS
    # needs its own rounded tile and transparent margin. Store/mobile assets
    # use the full square; Windows/Linux use a rounded tile without the margin.
    # `cargo tauri icon` rejects non-square sources, but the brand SVGs are
    # authored at 986x833. Pad the viewBox to a square canvas (centered, no
    # stretch) before generating so Windows/Linux builds do not fail.
    function Get-SquareSvg([string]$Path) {
        $raw = Get-Content -Raw -LiteralPath $Path
        $svg = [regex]::Match($raw, '<svg\b[^>]*>')
        if (-not $svg.Success) { return $Path }
        $box = [regex]::Match($svg.Value, 'viewBox="([^"]+)"')
        if (-not $box.Success) { return $Path }
        $n = @($box.Groups[1].Value -split '[\s,]+' |
            Where-Object { $_ -ne '' } | ForEach-Object { [double]$_ })
        if ($n.Count -ne 4) { return $Path }
        $w = $n[2]; $h = $n[3]
        if ([Math]::Abs($w - $h) -lt 0.01) { return $Path }
        $size = [Math]::Max($w, $h)
        $x = $n[0] - ($size - $w) / 2
        $y = $n[1] - ($size - $h) / 2
        $inv = [System.Globalization.CultureInfo]::InvariantCulture
        $tag = [regex]::Replace($svg.Value, 'viewBox="[^"]+"',
            ('viewBox="{0} {1} {2} {3}"' -f $x.ToString($inv), $y.ToString($inv),
                $size.ToString($inv), $size.ToString($inv)))
        $tag = [regex]::Replace($tag, 'width="[^"]+"', ('width="{0}"' -f $size.ToString($inv)))
        $tag = [regex]::Replace($tag, 'height="[^"]+"', ('height="{0}"' -f $size.ToString($inv)))
        $square = $raw.Substring(0, $svg.Index) + $tag +
            $raw.Substring($svg.Index + $svg.Length)
        $out = Join-Path ([System.IO.Path]::GetTempPath()) `
            ((Split-Path -Leaf $Path) -replace '\.svg$', '-square.svg')
        [System.IO.File]::WriteAllText($out, $square,
            (New-Object System.Text.UTF8Encoding($false)))
        return $out
    }

    $fullBleed = Get-SquareSvg (Resolve-Path "icons/app-icon.svg")
    $rounded = Get-SquareSvg (Resolve-Path "icons/app-icon-rounded.svg")
    $macos = Resolve-Path "icons/app-icon-macos.svg"
    $desktopNames = @(
        "icon.ico", "icon.png", "32x32.png", "64x64.png",
        "128x128.png", "128x128@2x.png"
    )
    $latestSource = ($fullBleed, $rounded, $macos, $PSCommandPath |
        Get-Item | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1).LastWriteTimeUtc
    $need = $false
    foreach ($name in ($desktopNames + @("icon.icns", "source.png"))) {
        $output = Join-Path "icons" $name
        if (-not (Test-Path $output) -or (Get-Item $output).LastWriteTimeUtc -lt $latestSource) {
            $need = $true
        }
    }
    if (-not $need) { return }

    $fullOut = Join-Path $PSScriptRoot "icons/.gen-full"
    $roundOut = Join-Path $PSScriptRoot "icons/.gen-rounded"
    $macOut = Join-Path $roundOut "macos"
    Remove-Item $fullOut, $roundOut -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $fullOut | Out-Null
    New-Item -ItemType Directory -Path $roundOut | Out-Null
    New-Item -ItemType Directory -Path $macOut | Out-Null
    cargo tauri icon $fullBleed -o $fullOut
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate square icons" }
    cargo tauri icon $rounded -o $roundOut
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate rounded icons" }
    cargo tauri icon $macos -o $macOut
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate macOS icons" }

    Copy-Item -Path (Join-Path $fullOut "*") -Destination "icons" -Recurse -Force
    foreach ($name in $desktopNames) {
        Copy-Item -Force (Join-Path $roundOut $name) (Join-Path "icons" $name)
    }
    Copy-Item -Force (Join-Path $macOut "icon.icns") "icons/icon.icns"
    cargo tauri icon $fullBleed --png 1024 -o $fullOut
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate source PNG" }
    Copy-Item -Force (Join-Path $fullOut "1024x1024.png") "icons/source.png"
    Remove-Item $fullOut, $roundOut -Recurse -Force
} finally {
    Pop-Location
}
