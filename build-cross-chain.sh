#!/bin/bash

set -e

init_variables() {
  # version configurations
  : "${CCTOOLS_VERSION:="973.0.1"}"
  : "${LINKER_VERSION:="609"}"
  : "${TAPI_VERSION:="1100.0.11"}"
  : "${CCTOOLS_PORT_VERSION:="$CCTOOLS_VERSION-ld64-$LINKER_VERSION"}"

  # default directory format
  # $PWD
  #  +- cctools - cctools git
  #  +- tapi - tapi git
  #  `- target -build output

  : "${CCTOOLS_SRC:="$PWD/cctools"}"
  : "${TAPI_SRC:="$PWD/tapi"}"
  : "${TARGET_DIR:="$PWD/target"}"

  # build performance setting
  : "${JOBS:="$(guess_threads || echo 1)"}"
  export JOBS

  echo "build tapi and ld by cctools-port by tpoechtrager" 1>&2
  echo "tapi and cctools-port are a part of osxcross" 1>&2
  echo "  https://github.com/tpoechtrager/apple-libtapi.git" 1>&2
  echo "  https://github.com/tpoechtrager/cctools-port.git" 1>&2
  echo "configurations:" 1>&2
  echo "  CCTOOLS_VERSION:      $CCTOOLS_VERSION" 1>&2
  echo "  LINKER_VERSION:       $LINKER_VERSION" 1>&2
  echo "  TAPI_VERSION:         $TAPI_VERSION" 1>&2
  echo "  CCTOOLS_PORT_VERSION: $CCTOOLS_PORT_VERSION" 1>&2
  echo "  CCTOOLS_SRC:          $CCTOOLS_SRC" 1>&2
  echo "  TAPI_SRC:             $TAPI_SRC" 1>&2
  echo "  TARGET_DIR:           $TARGET_DIR" 1>&2
  echo "  JOBS:                 $JOBS" 1>&2
  echo "" 1>&2
}

## download_git repo tag dest
## after this command, cwd will be in the repository.
## you can exit via popd
download_git() {
  mkdir -p "$3"
  pushd "$3"
  git init
  # use git config to allow overwrite
  git config --local remote.origin.url "$1"
  git config --local remote.origin.fetch +refs/heads/*:refs/remotes/origin/*
  git fetch origin "$2" --depth=1
  git checkout "$2"
}

install_dependencies() {
  export DEBIAN_FRONTEND=noninteractive;
  apt update && apt install -y \
    clang \
    cmake \
    llvm-dev \
    git \
    make \
    python
  unset DEBIAN_FRONTEND
}

build_tapi() {
  echo
  echo "################################"
  echo "########## build tapi ##########"
  echo "################################"
  echo

  download_git "https://github.com/tpoechtrager/apple-libtapi.git" "$TAPI_VERSION" "$TAPI_SRC"

  INSTALLPREFIX="$TARGET_DIR" ./build.sh
  ./install.sh

  popd
}

build_cctools() {
  echo
  echo "################################"
  echo "######### build cctools ########"
  echo "################################"
  echo

  download_git "https://github.com/tpoechtrager/cctools-port.git" "$CCTOOLS_PORT_VERSION" "$CCTOOLS_SRC"
  cd cctools

  ./configure \
    --prefix="$TARGET_DIR" \
    --target=x86_64-apple-darwin \
    --with-libtapi="$TARGET_DIR" \
    --disable-clang-as \
    --disable-lto-support

  for component in 3rd ld ;do
    pushd "ld64/src/$component"
      make -j"$JOBS"
    popd
  done
  pushd ld64/src/ld
    make install -j"$JOBS"
  popd 

  popd
}

guess_threads() {
  case $(uname -s) in
    Linux) grep -c ^processor /proc/cpuinfo ;;
    Darwin) sysctl -n hw.physicalcpu ;;
    BSD*) sysctl -n hw.ncpu ;;
    *) echo "can't guess thread count. please set \$JOBS" 1>&2; exit 1 ;;
  esac 
}

main() {
  init_variables
  install_dependencies
  build_tapi
  build_cctools
}

main
