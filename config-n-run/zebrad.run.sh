#!/bin/bash

# declare -x uid="$(whoami)"
declare -x uid="$(id -un)"
declare -x user="${uid:-"root"}"

echo -e "${user}:${user}"

sudo mkdir -v -p /var/log/zcash-zebrad/

sudo chown -v -R ${user}:${user} /var/log/zcash-zebrad/

# ./zebrad -c ./zebrad.toml connect-headers-only >> /var/log/zcash-zebrad/zebrad.log 2>&1 & disown

# ./zebrad connect-headers-only >> /var/log/zcash-zebrad/zebrad.log 2>&1 & disown

./zebrad -c ./zebrad.toml start-headers-only >> /var/log/zcash-zebrad/zebrad.log 2>&1 & disown

# ./zebrad start-headers-only >> /var/log/zcash-zebrad/zebrad.log 2>&1 & disown

