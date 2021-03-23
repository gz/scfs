#!/bin/bash
set -ex

wget https://github.com/vmware/differential-datalog/releases/download/v0.38.0/ddlog-v0.38.0-20210312081818-linux.tar.gz
tar zxvf ddlog-v0.38.0-20210312081818-linux.tar.gz

sudo apt-get install apt-mirror
cargo build

echo "Downloading 150 GiB of stuff, make sure you have enough space!"
sleep 5
apt-mirror ./apt-mirror.config

echo "Adjust PATH environment, add DDLOG_HOME:"
echo "export PATH=\$PATH:`pwd`/ddlog/bin"
echo "export DDLOG_HOME=`pwd`/ddlog"