# Construit le package Microsoft Store (.msix) directement depuis le build Tauri.
# Remplace la capture MSIX Packaging Tool, qui embarquait le profil WebView, des fichiers NVIDIA
# et un raccourci de désinstallation MSI cassé.
#
# Usage : npx tauri build ; puis powershell -File scripts/build-msix.ps1
# Le .msix n'est pas signé : c'est le Store qui le signe à la soumission.
param([string]$OutDir = "$PSScriptRoot\..\src-tauri\target\release\bundle\msix")
$ErrorActionPreference = 'Stop'

$root = Resolve-Path "$PSScriptRoot\.."
$exe = "$root\src-tauri\target\release\game-hub-widget.exe"
if (-not (Test-Path $exe)) { throw "game-hub-widget.exe introuvable : lancer d'abord 'npx tauri build'" }

# Version Store sur 4 chiffres : 1.1.0 -> 1.1.0.0
$version = (Get-Content "$root\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json).version + '.0'

$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.*\x64" -Directory |
    Where-Object { Test-Path "$($_.FullName)\makeappx.exe" } | Sort-Object Name -Descending | Select-Object -First 1
if (-not $sdk) { throw "Windows SDK introuvable (makeappx.exe / makepri.exe)" }

$stage = Join-Path $env:TEMP "ggh-msix-stage"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory $stage | Out-Null
Copy-Item $exe $stage
Copy-Item "$root\src-tauri\msix\Assets" "$stage\Assets" -Recurse
$manifest = (Get-Content "$root\src-tauri\msix\AppxManifest.xml" -Raw).Replace('{{VERSION}}', $version)
[System.IO.File]::WriteAllText("$stage\AppxManifest.xml", $manifest)  # UTF-8 sans BOM

# resources.pri : permet à Windows de choisir Assets\X.scale-200.png quand le manifeste dit Assets\X.png
$priConfig = Join-Path $env:TEMP "ggh-priconfig.xml"
& "$($sdk.FullName)\makepri.exe" createconfig /cf $priConfig /dq en-US /o | Out-Null
# Sans <packaging>, un seul resources.pri (sinon makepri le découpe par échelle pour des resource packs)
$cfg = [xml](Get-Content $priConfig -Raw)
$cfg.resources.SelectNodes('packaging') | ForEach-Object { [void]$cfg.resources.RemoveChild($_) }
$cfg.Save($priConfig)
& "$($sdk.FullName)\makepri.exe" new /pr $stage /cf $priConfig /of "$stage\resources.pri" /o | Out-Null
if ($LASTEXITCODE) { throw "makepri a échoué" }

New-Item -ItemType Directory $OutDir -Force | Out-Null
$msix = Join-Path (Resolve-Path $OutDir) "geewerrr.GeewersGameHub_${version}_x64.msix"
& "$($sdk.FullName)\makeappx.exe" pack /d $stage /p $msix /o | Out-Null
if ($LASTEXITCODE) { throw "makeappx a échoué" }

Remove-Item $stage -Recurse -Force
Write-Host "MSIX $version : $msix"
