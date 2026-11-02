# ==============================================================================
# DEADAIR // Automated Windows Source Installer (PowerShell)
# Sovereign offline-first Nostr station for blackouts and intranet survival
# ==============================================================================

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$ErrorActionPreference = "Stop"

$Repo = "deltafsociety/dead-air"
$GitUrl = "https://github.com/$Repo.git"
$ZipUrl = "https://github.com/$Repo/archive/refs/heads/master.zip"

Write-Host "
   ____  _________    ____  ___    ________ 
  / __ \/ ____/   |  / __ \/   |  /  _/ __ \
 / / / / __/ / /| | / / / / /| |  / // /_/ /
/ /_/ / /___/ ___ |/ /_/ / ___ |_/ // _, _/ 
/_____/_____/_/  |_/_____/_/  |_/___/_/ |_|  
" -ForegroundColor Cyan

Write-Host ">> DEADAIR Native Windows Source Installer" -ForegroundColor Green
Write-Host "   Target: Building native Windows binary from source with embedded cybernetic icon"
Write-Host ""

# ------------------------------------------------------------------------------
# 1. Edition Selection (Interactive or via $env:DEADAIR_TARGET)
# ------------------------------------------------------------------------------
$TargetChoice = $env:DEADAIR_TARGET
if (-not $TargetChoice) {
    if ([Environment]::UserInteractive -and -not [Console]::IsInputRedirected) {
        Write-Host "Please select the DEADAIR edition you want to install:" -ForegroundColor Cyan
        Write-Host "  [1] Terminal TUI Station (command: deadair) [DEFAULT]"
        Write-Host "  [2] Desktop GUI Station (adds to Start Menu & Desktop with custom icon)"
        Write-Host "  [3] Both (Terminal TUI + Desktop GUI)"
        Write-Host ""
        $ChoiceInput = Read-Host "Enter choice [1/2/3] (default: 1)"
        if ($ChoiceInput) { $TargetChoice = $ChoiceInput.Trim() } else { $TargetChoice = "1" }
    } else {
        $TargetChoice = "1"
    }
}

$BuildTui = ($TargetChoice -eq "1" -or $TargetChoice -eq "3" -or $TargetChoice -eq "tui" -or $TargetChoice -eq "both" -or $TargetChoice -eq "all")
$BuildGui = ($TargetChoice -eq "2" -or $TargetChoice -eq "3" -or $TargetChoice -eq "gui" -or $TargetChoice -eq "both" -or $TargetChoice -eq "all")

if ($BuildTui -and $BuildGui) {
    Write-Host "[+] Selected: Both Terminal TUI and Desktop GUI" -ForegroundColor Green
} elseif ($BuildGui) {
    Write-Host "[+] Selected: Desktop GUI Station" -ForegroundColor Green
} else {
    Write-Host "[+] Selected: Terminal TUI Station" -ForegroundColor Green
}
Write-Host ""

# ------------------------------------------------------------------------------
# 2. Architecture Check
# ------------------------------------------------------------------------------
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
Write-Host "[+] Platform: Windows ($Arch)" -ForegroundColor Gray

# ------------------------------------------------------------------------------
# 3. Rust Toolchain (Cargo & Rustc) Check / Install
# ------------------------------------------------------------------------------
$CargoBin = "$env:USERPROFILE\.cargo\bin"
if ($env:Path -split ';' -notcontains $CargoBin) {
    $env:Path = "$CargoBin;$env:Path"
}

$HasCargo = (Get-Command cargo.exe -ErrorAction SilentlyContinue) -ne $null
$HasRustc = (Get-Command rustc.exe -ErrorAction SilentlyContinue) -ne $null

if (-not $HasCargo -or -not $HasRustc) {
    Write-Host "[!] Rust toolchain not detected on system." -ForegroundColor Yellow
    Write-Host "[+] Downloading rustup-init for Windows..." -ForegroundColor Cyan

    $RustupInitUrl = if ($Arch -eq "x64") {
        "https://win.rustup.rs/x86_64"
    } else {
        "https://win.rustup.rs/i686"
    }

    $TempRustup = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $RustupInitUrl -OutFile $TempRustup -UseBasicParsing

    Write-Host "[+] Installing Rust stable (minimal profile)..." -ForegroundColor Cyan
    Start-Process -FilePath $TempRustup -ArgumentList "-y", "--default-toolchain", "stable", "--profile", "minimal" -Wait -NoNewWindow
    Remove-Item $TempRustup -Force -ErrorAction SilentlyContinue

    if ($env:Path -split ';' -notcontains $CargoBin) {
        $env:Path = "$CargoBin;$env:Path"
    }
}

# Verify cargo is available
if (-not (Get-Command cargo.exe -ErrorAction SilentlyContinue)) {
    Write-Host "[-] Error: Cargo is still not accessible in PATH ($CargoBin)." -ForegroundColor Red
    Write-Host "    Please restart your terminal or install Rust manually from https://rustup.rs"
    exit 1
}

$RustVersion = (& rustc --version)
Write-Host "[+] Using Rust toolchain: $RustVersion" -ForegroundColor Green

# ------------------------------------------------------------------------------
# 4. Source Retrieval (Local workspace or Git clone)
# ------------------------------------------------------------------------------
$SourceDir = ""
$CleanupNeeded = $false
$TempDir = "$env:TEMP\deadair-build-$(Get-Random)"

if ((Test-Path "Cargo.toml") -and (Select-String -Path "Cargo.toml" -Pattern 'name = "deadair"' -Quiet)) {
    Write-Host "[+] Found local DEADAIR source tree in current directory." -ForegroundColor Cyan
    $SourceDir = (Get-Item .).FullName
} elseif ((Test-Path "..\Cargo.toml") -and (Select-String -Path "..\Cargo.toml" -Pattern 'name = "deadair"' -Quiet)) {
    Write-Host "[+] Found local DEADAIR source tree in parent directory." -ForegroundColor Cyan
    $SourceDir = (Get-Item ..).FullName
} else {
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
    $CleanupNeeded = $true
    $SourceDir = "$TempDir\source"

    $HasGit = (Get-Command git.exe -ErrorAction SilentlyContinue) -ne $null
    if ($HasGit) {
        Write-Host "[+] Cloning latest DEADAIR source from $Repo..." -ForegroundColor Cyan
        & git clone --depth 1 $GitUrl $SourceDir
    } else {
        Write-Host "[+] Git not detected. Downloading source ZIP archive from GitHub..." -ForegroundColor Cyan
        $ZipFile = "$TempDir\source.zip"
        Invoke-WebRequest -Uri $ZipUrl -OutFile $ZipFile -UseBasicParsing
        Expand-Archive -Path $ZipFile -DestinationPath $TempDir -Force
        $Extracted = (Get-ChildItem -Path $TempDir -Directory | Where-Object { $_.Name -like "dead-air-*" } | Select-Object -First 1).FullName
        Rename-Item -Path $Extracted -NewName "source"
    }
}

# ------------------------------------------------------------------------------
# 5. Compile DEADAIR from Source
# ------------------------------------------------------------------------------
Push-Location $SourceDir
try {
    if ($BuildTui) {
        Write-Host "[+] Compiling Terminal TUI Station from source (release profile)..." -ForegroundColor Cyan
        Write-Host "    Embedding native Windows .ico resource into deadair.exe..." -ForegroundColor Gray
        & cargo build --release -p deadair
        if ($LASTEXITCODE -ne 0) { throw "Cargo compilation failed for deadair TUI" }
    }

    if ($BuildGui) {
        Write-Host "[+] Compiling Desktop GUI Station from source (release profile)..." -ForegroundColor Cyan
        Write-Host "    Embedding native Windows .ico resource into deadair-gui.exe..." -ForegroundColor Gray
        & cargo build --release -p deadair-gui
        if ($LASTEXITCODE -ne 0) { throw "Cargo compilation failed for deadair-gui" }
    }
} finally {
    Pop-Location
}

Write-Host "[+] Build succeeded!" -ForegroundColor Green

# ------------------------------------------------------------------------------
# 6. Install Binaries & Add to User PATH
# ------------------------------------------------------------------------------
$InstallDir = "$env:LOCALAPPDATA\deadair\bin"
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

$IconSrc = "$SourceDir\assets\icon.ico"
$TargetIcon = "$env:LOCALAPPDATA\deadair\icon.ico"
if (Test-Path $IconSrc) {
    Copy-Item -Path $IconSrc -Destination $TargetIcon -Force
}

$WshShell = New-Object -ComObject WScript.Shell
$ProgramsDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)
$DesktopDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Desktop)

# Install TUI
if ($BuildTui) {
    $TuiCompiled = "$SourceDir\target\release\deadair.exe"
    $TuiDest = "$InstallDir\deadair.exe"
    Copy-Item -Path $TuiCompiled -Destination $TuiDest -Force
    Write-Host "[+] Installed Terminal TUI: $TuiDest" -ForegroundColor Green
}

# Install GUI & Create Shortcuts
if ($BuildGui) {
    $GuiCompiled = "$SourceDir\target\release\deadair-gui.exe"
    $GuiDest = "$InstallDir\deadair-gui.exe"
    Copy-Item -Path $GuiCompiled -Destination $GuiDest -Force
    Write-Host "[+] Installed Desktop GUI: $GuiDest" -ForegroundColor Green

    try {
        # Start Menu shortcut
        $StartMenuShortcut = "$ProgramsDir\DEADAIR.lnk"
        $Shortcut = $WshShell.CreateShortcut($StartMenuShortcut)
        $Shortcut.TargetPath = $GuiDest
        if (Test-Path $TargetIcon) {
            $Shortcut.IconLocation = "$TargetIcon,0"
        }
        $Shortcut.Description = "DEADAIR // Sovereign Offline Nostr Station"
        $Shortcut.WorkingDirectory = "$env:USERPROFILE"
        $Shortcut.Save()
        Write-Host "[+] Added DEADAIR to Windows Start Menu!" -ForegroundColor Green

        # Desktop shortcut
        $DesktopShortcut = "$DesktopDir\DEADAIR.lnk"
        $ShortcutDesk = $WshShell.CreateShortcut($DesktopShortcut)
        $ShortcutDesk.TargetPath = $GuiDest
        if (Test-Path $TargetIcon) {
            $ShortcutDesk.IconLocation = "$TargetIcon,0"
        }
        $ShortcutDesk.Description = "DEADAIR // Sovereign Offline Nostr Station"
        $ShortcutDesk.WorkingDirectory = "$env:USERPROFILE"
        $ShortcutDesk.Save()
        Write-Host "[+] Added DEADAIR shortcut to Desktop!" -ForegroundColor Green
    } catch {
        Write-Host "[!] Note: Could not create shortcuts: $_" -ForegroundColor Yellow
    }
}

# Add InstallDir to User PATH permanently if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if (-not $UserPath) { $UserPath = "" }
$Paths = $UserPath -split ';' | Where-Object { $_ -ne "" }

if ($Paths -notcontains $InstallDir) {
    Write-Host "[+] Adding $InstallDir to User PATH..." -ForegroundColor Cyan
    $NewPath = if ($UserPath.Length -gt 0) { "$UserPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$InstallDir;$env:Path"
}

# ------------------------------------------------------------------------------
# 7. Cleanup
# ------------------------------------------------------------------------------
if ($CleanupNeeded -and (Test-Path $TempDir)) {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

# ------------------------------------------------------------------------------
# 8. Summary & Instructions
# ------------------------------------------------------------------------------
Write-Host ""
Write-Host "================================================================" -ForegroundColor Green
Write-Host "  DEADAIR has been successfully built and installed from source! " -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Green
Write-Host ""
if ($BuildTui) {
    Write-Host "  * Terminal TUI: Run anytime in PowerShell / CMD: " -NoNewline
    Write-Host "deadair" -ForegroundColor Cyan
}
if ($BuildGui) {
    Write-Host "  * Desktop GUI:  Launch from Start Menu, Desktop, or run: " -NoNewline
    Write-Host "deadair-gui" -ForegroundColor Cyan
}
Write-Host ""
Write-Host "Quick Start:"
Write-Host "  1. Choose a handle:      " -NoNewline; Write-Host "/name YourNick" -ForegroundColor Yellow
Write-Host "  2. Start domestic relay: " -NoNewline; Write-Host "[r]" -ForegroundColor Yellow
Write-Host "  3. Share domestic IP:    " -NoNewline; Write-Host "[s]" -ForegroundColor Yellow
Write-Host ""
Write-Host "Enjoy sovereign and blackout-proof intranet communications."
