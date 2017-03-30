#!/bin/bash
set -ex

coverage() {
  wget -c https://github.com/SimonKagstrom/kcov/archive/master.tar.gz &&
  tar xzf master.tar.gz &&
  cd kcov-master &&
  mkdir build &&
  cd build &&
  cmake .. -DCMAKE_INSTALL_PREFIX=~/.local &&
  make &&
  make install &&
  cd ../.. &&
  export PATH=$PATH:~/.local/bin &&
  for file in $(find target/debug -maxdepth 1 -name "swindon*-*" -not -name "*-dev" -executable); do
    echo "Running ${file}" &&
    mkdir -p "target/cov/$(basename $file)" &&
    kcov --include-path=$(pwd) --verify "target/cov/$(basename $file)/" "$file" || exit 1
  done &&
  bash <(curl -s https://codecov.io/bash) &&
  echo "Uploaded code coverage"
} && coverage
