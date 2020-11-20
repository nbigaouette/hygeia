# Use newest TLS1.2 protocol version for HTTPS connections
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12


$githubBaseUrl = "https://github.com/nbigaouette/hygeia"
$githubApiUrl = "https://api.github.com/repos/nbigaouette/hygeia"


# ----------------------------------------------------------------------
function RefreshEnvironmentVariables {
    # Update the script's PATH environment variable to find the installed binary
    # See https://stackoverflow.com/a/31845512
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

    # Refresh environment variables
    refreshenv
}

# ----------------------------------------------------------------------
function NewTemporaryDirectory {
    $parent = [System.IO.Path]::GetTempPath()
    [string] $name = [System.Guid]::NewGuid()
    $newDirectory = Join-Path $parent $name
    New-Item -ItemType Directory -Path $newDirectory | Out-Null

    $newDirectory
}


# ----------------------------------------------------------------------
function IsRunningUnderWindows() {
    $detectedPlatform = [System.Environment]::OSVersion.Platform
    Switch ($detectedPlatform) {
        "Unix"      { $false }
        "Win32NT"   { $true }
        default     { Write-Error -ErrorAction Stop "Failed to detect platform: $detectedPlatform"}
    }
}


# ----------------------------------------------------------------------
function DetectPlatform {

    if (IsRunningUnderWindows) {
        $platform = "pc-windows-msvc"
    }
    else {
        $uname_platform = uname -s | tr '[:upper:]' '[:lower:]'

        $platform = Switch -Wildcard ($uname_platform) {
            "linux"     {"unknown-linux-musl"}
            "darwin"    {"apple-darwin"}
            "msys_nt*"  {"pc-windows-msvc"}
            "mingw*"    {"pc-windows-msvc"}
            default     { Write-Error -ErrorAction Stop "Failed to detect platform" }
        }
    }

    Write-Host "Detected platform: $platform"

    $platform
}


# ----------------------------------------------------------------------
function GetDownloadUrl() {
    $platform = DetectPlatform

    # Fetch the latest release's information
    $latestRelease = Invoke-RestMethod -Method Get -Uri "$githubApiUrl/releases/latest"

    # Find the 'browser_download_url' that matches the detected platform
    $latestRelease.assets.browser_download_url -match $platform
}


# ----------------------------------------------------------------------
function Main() {
    Write-Host ""
    Write-Host "Installing Hygeia, please wait..."
    Write-Host ""

    $tmpDir = NewTemporaryDirectory
    $tmpArchive = Join-Path $tmpDir hygeia.zip
    # We'll append '.exe' for Windows later
    $tmpBinary = Join-Path $tmpDir hygeia

    Write-Host "Downloading to $tmpDir..."

    $downloadUrl = GetDownloadUrl

    Invoke-RestMethod -Method Get -Uri $downloadUrl -OutFile $tmpArchive

    Expand-Archive -LiteralPath $tmpArchive -DestinationPath $tmpDir

    if (IsRunningUnderWindows) {
        $tmpBinary = "$tmpBinary.exe"
    }
    else {
        # Make executable as 'Expand-Archive' looses executable bit
        & chmod +x $tmpBinary
    }

    & $tmpBinary setup powershell
}


# ----------------------------------------------------------------------
Main

Write-Host ""
Write-Host "Installation done. Please restart your shell to get access to 'hygeia' command!"
