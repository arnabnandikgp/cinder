#!/usr/bin/env bash
# Surfpool mainnet fork on :8899. ER/QFS remain pointed at localhost.
set -euo pipefail

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

if [[ -z "${SURFPOOL_RPC_URL:-}" ]]; then
  echo "error: SURFPOOL_RPC_URL must point to a mainnet RPC" >&2
  exit 1
fi

SURFPOOL_VERSION="1.5.0"
# SHA256 of GitHub release tarballs for v1.5.0 (computed from the published assets).
SURFPOOL_SHA_DARWIN_ARM64="418d1464050cb1e09225b37b907952383f712648544318a0ca7ce13304e7915a"
SURFPOOL_SHA_LINUX_X64="5b20a3b46e60c4f819af7b4da5c3ea211f76041710617841cc23247d15887ddc"

install_surfpool() {
  local os arch tar sha url tmp
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}-${arch}" in
    Darwin-arm64)
      tar="surfpool-darwin-arm64.tar.gz"
      sha="${SURFPOOL_SHA_DARWIN_ARM64}"
      ;;
    Linux-x86_64)
      tar="surfpool-linux-x64.tar.gz"
      sha="${SURFPOOL_SHA_LINUX_X64}"
      ;;
    *)
      echo "error: no pinned Surfpool ${SURFPOOL_VERSION} build for ${os}-${arch}" >&2
      echo "  download from https://github.com/solana-foundation/surfpool/releases/tag/v${SURFPOOL_VERSION}" >&2
      exit 1
      ;;
  esac

  url="https://github.com/solana-foundation/surfpool/releases/download/v${SURFPOOL_VERSION}/${tar}"
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' RETURN
  echo "stack-fork: downloading ${url}"
  curl -fsSL -o "${tmp}/${tar}" "${url}"
  if command -v sha256sum >/dev/null 2>&1; then
    echo "${sha}  ${tmp}/${tar}" | sha256sum -c -
  else
    echo "${sha}  ${tmp}/${tar}" | shasum -a 256 -c -
  fi
  tar -xzf "${tmp}/${tar}" -C "${tmp}"
  if [[ ! -f "${tmp}/surfpool" ]]; then
    echo "error: tarball did not contain ./surfpool" >&2
    exit 1
  fi
  mkdir -p "${HOME}/.local/bin"
  install -m 0755 "${tmp}/surfpool" "${HOME}/.local/bin/surfpool"
  if [[ "${os}" == "Darwin" ]]; then
    xattr -d com.apple.quarantine "${HOME}/.local/bin/surfpool" 2>/dev/null || true
  fi
}

if ! command -v surfpool >/dev/null 2>&1; then
  echo "stack-fork: installing surfpool ${SURFPOOL_VERSION} from GitHub releases..."
  install_surfpool
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
fi

if ! command -v surfpool >/dev/null 2>&1; then
  echo "error: surfpool not on PATH after install" >&2
  exit 1
fi

echo "stack-fork: starting Surfpool with SURFPOOL_RPC_URL"
echo "  RPC :8899  (point local ER/QFS remotes here)"
echo "  Do not send-register-ixs to Phoenix mainnet; send built ixs to this fork."
# Local fork only: skip-sig lets venue-boot pad the Phoenix onboarder signature.
exec surfpool start \
  --rpc-url "${SURFPOOL_RPC_URL}" \
  --skip-signature-verification \
  --ci \
  --no-deploy
