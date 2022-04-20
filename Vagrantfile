$script = <<-SCRIPT
sudo apk update
sudo apk add qemu-img openssl ncurses-libs libgcc nbd nbd-client
sudo -u vagrant curl --proto '=https' --tlsv1.2 -sSf -o /home/vagrant/sh.rustup.rs https://sh.rustup.rs
sudo -u vagrant sh /home/vagrant/sh.rustup.rs -y
sudo -u vagrant rm /home/vagrant/sh.rustup.rs
sudo apk add docker
sudo addgroup vagrant docker
sudo rc-update add docker boot
sudo service docker start
sudo apk add docker-cli-compose
SCRIPT

Vagrant.configure("2") do |config|
  config.vm.box = "generic/alpine316"
  config.vm.provision "shell", inline: $script

  # override some defaults
  config.vbguest.auto_update = false
  config.vbguest.no_install = true
  config.vm.box_check_update = false

  # forward port for nbd server
  config.vm.network "forwarded_port", guest: 10809, host: 10809
end
