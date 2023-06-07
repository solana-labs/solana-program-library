#!/usr/bin/env bash

# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
# WARNING: THIS IS NOT TO BE USED LOCALLY, BUT ONLY IN GITHUB ACTIONS TO CLEAR
# DISK SPACE ON UBUNTU. IT IS NOT MARKED AS EXECUTABLE BY DEFAULT FOR SAFETY AND
# ONLY RUNS IN CI.
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
if [[ -n "$CI" ]]; then
  sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc
  sudo apt purge aria2 ansible azure-cli shellcheck rpm xorriso zsync \
    clang-6.0 lldb-6.0 lld-6.0 clang-format-6.0 \
    clang-8 lldb-8 lld-8 clang-format-8 \
    clang-9 lldb-9 lld-9 clangd-9 clang-format-9 \
    dotnet-sdk-* \
    esl-erlang \
    firefox \
    g++-8 g++-9 \
    gfortran-8 gfortran-9 \
    google-chrome-stable google-cloud-sdk \
    ghc-* \
    cabal-install-* \
    heroku \
    imagemagick libmagickcore-dev libmagickwand-dev libmagic-dev \
    ant ant-optional \
    kubectl \
    mercurial mono-complete \
    mysql-client libmysqlclient-dev mysql-server \
    mssql-tools unixodbc-dev yarn bazel chrpath libssl-dev libxft-dev \
    libfreetype6 libfreetype6-dev libfontconfig1 libfontconfig1-dev \
    php* \
    snmp pollinate \
    libpq-dev postgresql-client \
    powershell ruby-full \
    sphinxsearch subversion mongodb-org -yq >/dev/null 2>&1
  sudo apt-get autopurge -y >/dev/null 2>&1
  sudo apt-get autoclean -y >/dev/null 2>&1
fi
