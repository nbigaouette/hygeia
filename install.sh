#!/bin/sh

set -e -o nounset -o pipefail


GITHUB_BASE_URL="https://github.com/nbigaouette/hygeia"
GITHUB_API_URL="https://api.github.com/repos/nbigaouette/hygeia"

BOLD="$(tput bold 2>/dev/null || echo '')"
GREY="$(tput setaf 0 2>/dev/null || echo '')"
UNDERLINE="$(tput smul 2>/dev/null || echo '')"
RED="$(tput setaf 1 2>/dev/null || echo '')"
GREEN="$(tput setaf 2 2>/dev/null || echo '')"
YELLOW="$(tput setaf 3 2>/dev/null || echo '')"
BLUE="$(tput setaf 4 2>/dev/null || echo '')"
MAGENTA="$(tput setaf 5 2>/dev/null || echo '')"
NO_COLOR="$(tput sgr0 2>/dev/null || echo '')"


info() {
    printf "%s\n" "${BOLD}${GREY}>${NO_COLOR} $*"
}


warn() {
    printf "%s\n" "${YELLOW}! $*${NO_COLOR}"
}


error() {
    printf "%s\n" "${RED}x $*${NO_COLOR}" >&2
}


complete() {
    printf "%s\n" "${GREEN}✓${NO_COLOR} $*"
}


detect_platform() {
    local platform
    platform="$(uname -s | tr '[:upper:]' '[:lower:]')"

    case "${platform}" in
        msys_nt*) platform="pc-windows-msvc" ;;
        # mingw is Git-Bash
        mingw*) platform="pc-windows-msvc" ;;
        # use the statically compiled musl bins on linux to avoid linking issues.
        linux) platform="unknown-linux-musl" ;;
        darwin) platform="apple-darwin" ;;
    esac

    echo "${platform}"
}


get_download_url() {
    platform=$(detect_platform)
    fetch "${GITHUB_API_URL}/releases/latest" | grep browser_download_url | sed "s|.*: ||g" | sed 's|"||g' | grep ${platform}
}


# Gets path to a temporary directory
get_tmpdir() {
    if hash mktemp; then
        mktemp -d
    else
        # No really good options here--let's pick a default + hope
        d="/tmp/hygeia"
        mkdir -p ${d}
        echo ${d}
    fi
}


fetch() {
    local command

    if hash curl 2>/dev/null; then
        set +e
        command="curl --silent --fail --location $1"
        curl --silent --fail --location "$1"
        rc=$?
        set -e
    else
        if hash wget 2>/dev/null; then
            set +e
            command="wget -O- -q $1"
            wget -O- -q "$1"
            rc=$?
            set -e
        else
            error "No HTTP download program (curl, wget) found…"
            exit 1
        fi
    fi

    if [ $rc -ne 0 ]; then
        printf "\n" >&2
        error "Command failed (exit code $rc): ${BLUE}${command}${NO_COLOR}"
        printf "\n" >&2
        info "Please create an issue:" >&2
        info "${BOLD}${UNDERLINE}${GITHUB_BASE_URL}/issues/new/${NO_COLOR}\n" >&2
        exit $rc
    fi
}


setup_shell() {
    binary="${1}"
    shell="${2}"

    cmd="${tmpbin} setup ${shell}"
    info ${cmd}
    ${cmd}
}


main() {
    local url
    local tmpdir
    local tmparchive
    local tmpbin

    printf "\n"
    info "Installing Hygeia, please wait…"

    url=$(get_download_url)
    tmpdir="$(get_tmpdir)"
    tmparchive="${tmpdir}/hygeia.zip"
    tmpbin="${tmpdir}/hygeia"

    # According to https://unix.stackexchange.com/q/2690, zip files cannot be read
    # through a pipe. We'll have to do our own file-based setup.
    fetch "${url}" >"${tmparchive}"
    unzip "${tmparchive}" -d "${tmpdir}"

    setup_shell "${tmpbin}" "bash"
    setup_shell "${tmpbin}" "zsh"

    complete "Hygeia installation successful!"
}


main
