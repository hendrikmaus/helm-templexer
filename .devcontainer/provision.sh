#!/bin/sh

# rust toolchain and optional additional tools
rustc --version
rustup component add clippy rustfmt

# if you want to use some additional tolling
# you might want to grant the container some more cpu cycles though
# see hostRequirements.cpus in the devcontainer definition file
#cargo install cargo-watch cargo-nextest

# install latest available version of helm
curl https://baltocdn.com/helm/signing.asc | gpg --dearmor | sudo tee /usr/share/keyrings/helm.gpg > /dev/null
sudo apt-get install apt-transport-https --yes
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/helm.gpg] https://baltocdn.com/helm/stable/debian/ all main" | sudo tee /etc/apt/sources.list.d/helm-stable-debian.list
sudo apt-get update
sudo apt-get install helm
