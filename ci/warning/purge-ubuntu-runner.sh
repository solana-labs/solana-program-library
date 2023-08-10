#!/usr/bin/env bash

# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
# WARNING: THIS IS NOT TO BE USED LOCALLY, BUT ONLY IN GITHUB ACTIONS TO CLEAR
# DISK SPACE ON UBUNTU. IT IS NOT MARKED AS EXECUTABLE BY DEFAULT FOR SAFETY AND
# ONLY RUNS IN CI.
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
if [[ -n "$CI" ]]; then
  set -e
  # Clears 4GB
  sudo docker rmi $(docker image ls -aq)
  # Clears 12GB
  sudo rm -rf /usr/local/lib/android
  # Clears 7GB
  sudo apt purge azure-cli \
    dotnet-sdk-* \
    firefox \
    g++-9 \
    gfortran-9 \
    google-chrome-stable google-cloud-sdk \
    ant ant-optional \
    mercurial \
    mono-complete \
    mysql-client libmysqlclient-dev mysql-server \
    mssql-tools unixodbc-dev \
    libfreetype6 libfreetype6-dev libfontconfig1 libfontconfig1-dev \
    shim-signed \
    nginx \
    php* \
    libpq-dev \
    powershell \
    ruby-full \
    sphinxsearch \
    subversion \
    -yq --allow-remove-essential
  sudo apt autopurge -y
  sudo apt autoclean -y
  # Clear extra dirs
  sudo rm -rf /usr/share/dotnet \
    /usr/share/php \
    /etc/mono \
    /usr/lib/mono \
    /etc/mysql
fi
