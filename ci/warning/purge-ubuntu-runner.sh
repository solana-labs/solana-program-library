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
  # Clears 6GB
  sudo apt update
  sudo apt purge ansible \
    azure-cli \
    dotnet-sdk-* \
    firefox \
    g++-9 \
    gfortran-9 \
    google-chrome-stable google-cloud-sdk \
    ant ant-optional \
    imagemagick* \
    mercurial \
    mono-complete \
    mysql-client libmysqlclient-dev mysql-server \
    mssql-tools unixodbc-dev libxft-dev \
    libfreetype6 libfreetype6-dev libfontconfig1 libfontconfig1-dev \
    nginx \
    php* \
    libpq-dev postgresql-client \
    powershell \
    ruby-full \
    sphinxsearch \
    -yq
  sudo apt autopurge -y
  sudo apt autoclean -y
fi
