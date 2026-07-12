# Maintainer: Your Name <your@email.com>

pkgname=os-switch
pkgver=0.2.1
pkgrel=1
pkgdesc="双系统快速切换工具，支持休眠切换和直接切换"
arch=('x86_64')
url="https://github.com/wuzhizhe7273/os-switch"
license=('MIT')
options=(!debug)
makedepends=('cargo' 'git')
source=("git+${url}#tag=v${pkgver}")
sha256sums=('SKIP')

build() {
    cd "${pkgname}"
    cargo build --release --locked
}

package() {
    cd "${pkgname}"
    install -Dm755 target/release/os-switch "$pkgdir/usr/bin/os-switch"
}
