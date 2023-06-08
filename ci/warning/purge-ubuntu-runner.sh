#!/usr/bin/env bash

# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
# WARNING: THIS IS NOT TO BE USED LOCALLY, BUT ONLY IN GITHUB ACTIONS TO CLEAR
# DISK SPACE ON UBUNTU. IT IS NOT MARKED AS EXECUTABLE BY DEFAULT FOR SAFETY AND
# ONLY RUNS IN CI.
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
if [[ -n "$CI" ]]; then
  # Clears 4GB
  sudo docker rmi $(docker image ls -aq)
  # Clears 12GB
  sudo rm -rf /usr/local/lib/android
  # Clears ?
  sudo apt update
  sudo apt install -y shim-signed shim grub2-common grub-efi-amd64-signed
  sudo apt purge aria2 \
    ansible \
    azure-cli \
    xorriso \
    dotnet-sdk-* \
    firefox \
    g++-9 \
    gfortran-9 \
    google-chrome-stable google-cloud-sdk \
    ant ant-optional \
    mercurial \
    mono-complete \
    mysql-client libmysqlclient-dev mysql-server \
    mssql-tools unixodbc-dev libxft-dev \
    libfreetype6 libfreetype6-dev libfontconfig1 libfontconfig1-dev \
    nginx \
    shim-signed \
    php* \
    libpq-dev postgresql-client \
    powershell \
    ruby-full \
    sphinxsearch \
    subversion \
    -yq --allow-remove-essential
  sudo apt autopurge -y
  sudo apt autoclean -y
fi
