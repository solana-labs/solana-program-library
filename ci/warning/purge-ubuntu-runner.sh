#!/usr/bin/env bash

# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
# WARNING: THIS IS NOT TO BE USED LOCALLY, BUT ONLY IN GITHUB ACTIONS TO CLEAR
# DISK SPACE ON UBUNTU. IT IS NOT MARKED AS EXECUTABLE BY DEFAULT FOR SAFETY AND
# ONLY RUNS IN CI.
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
if [[ -n "$CI" ]]; then
  sudo docker rmi $(docker image ls -aq)
  sudo apt install grub2-common # for shim-signed
  sudo apt purge aria2 \
    ansible \
    azure-cli \
    xorriso \
    rpm \
    zsync \
    dotnet-sdk-* \
    firefox \
    g++-8 g++-9 \
    gfortran-9 \
    google-chrome-stable google-cloud-sdk \
    imagemagick libmagickcore-dev libmagickwand-dev libmagic-dev \
    ant ant-optional \
    mercurial mono-complete \
    mysql-client libmysqlclient-dev mysql-server \
    mssql-tools unixodbc-dev yarn chrpath libssl-dev libxft-dev \
    libfreetype6 libfreetype6-dev libfontconfig1 libfontconfig1-dev \
    nginx \
    php* \
    snmp pollinate \
    libpq-dev postgresql-client \
    powershell ruby-full \
    sphinxsearch subversion -yq
  sudo apt autopurge -y
  sudo apt autoclean -y
fi
