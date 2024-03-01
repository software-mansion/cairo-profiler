#!/bin/sh

REPO="https://github.com/software-mansion/cairo-profiler"
BINARY_NAME="cairo-profiler"
LOCAL_BIN="${HOME}/.local/bin"

main () {
  check_cmd curl
  check_cmd tar

  ls

  version=${1:-latest}
  release_tag=$(curl -# --fail -Ls -H 'Accept: application/json' "${REPO}/releases/{$version}" | sed -e 's/.*"tag_name":"\([^"]*\)".*/\1/')

  if [ -z "$release_tag" ]; then
    echo "No such version $version, please pass correct one (e.g. v0.1.0)"
    exit 1
  fi

  download_and_extract_binary "$release_tag"

  add_binary_to_path

  echo "${BINARY_NAME} (${release_tag}) has been installed successfully."
}

check_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
      echo "$1 is not available"
      echo "Please install $1 and run the script again"
      exit 1
  fi
}

download_and_extract_binary() {
  release_tag=$1

  # Define the operating system and architecture
  get_architecture
  _arch="$RETVAL"

  artifact_name=${BINARY_NAME}-${release_tag}-${_arch}

  echo "Downloading and extracting ${artifact_name}..."
  # Create a temporary directory
  temp_dir=$(mktemp -d)

  # Download and extract the archive
  curl -L "${REPO}/releases/download/${release_tag}/${artifact_name}.tar.gz" | tar -xz -C "${temp_dir}"

  # Move the binary to a LOCAL_BIN directory
  mkdir -p "${LOCAL_BIN}"
  mv "${temp_dir}/${artifact_name}/bin/${BINARY_NAME}" "${LOCAL_BIN}"

  # Clean up temporary files
  rm -rf "${temp_dir}"
}

get_architecture() {
  _ostype="$(uname -s)"
  _cputype="$(uname -m)"
  _clibtype="gnu"

  if [ "$_ostype" = Linux ] && ldd --_requested_version 2>&1 | grep -q 'musl'; then
    _clibtype="musl"
  fi

  if [ "$_ostype" = Darwin ] && [ "$_cputype" = i386 ] && sysctl hw.optional.x86_64 | grep -q ': 1'; then
    _cputype=x86_64
  fi

  case "$_ostype" in
  Linux)
    _ostype=unknown-linux-$_clibtype
    ;;

  Darwin)
    _ostype=apple-darwin
    ;;
  *)
    err "unsupported OS type: $_ostype"
    ;;
  esac

  case "$_cputype" in
  aarch64 | arm64)
    _cputype=aarch64
    ;;

  x86_64 | x86-64 | x64 | amd64)
    _cputype=x86_64
    ;;
  *)
    err "unknown CPU type: $_cputype"
    ;;
  esac

  _arch="${_cputype}-${_ostype}"

  RETVAL="$_arch"
}

add_binary_to_path() {
  # Store the correct profile file (i.e. .profile for bash or .zshenv for ZSH).
  case $SHELL in
  */zsh)
      PROFILE=${ZDOTDIR-"$HOME"}/.zshenv
      ;;
  */bash)
      PROFILE=$HOME/.bashrc
      ;;
  */fish)
      PROFILE=$HOME/.config/fish/config.fish
      ;;
  */ash | */sh)
      PROFILE=$HOME/.profile
      ;;
  *)
      echo "cairo-profiler: could not detect shell, manually add ${LOCAL_BIN} to your PATH."
      exit 0
  esac

  # Only add cairo-profiler if it isn't already in PATH.
  case ":$PATH:" in
      *":${LOCAL_BIN}/${BINARY_NAME}:"*)
          # The path is already in PATH, do nothing
          ;;
      *)
          # Add the universal-sierra-compiler directory to the path
          echo >> "$PROFILE" && echo "export PATH=\"\$PATH:$LOCAL_BIN\"" >> "$PROFILE"
          ;;
  esac
}

set -e
main "$@"
