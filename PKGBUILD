# Maintainer: Your Name <your@email.com>

pkgname=os-switch
pkgver=0.2.0
pkgrel=1
pkgdesc="双系统快速切换工具，支持休眠切换和直接切换"
arch=('x86_64')
url="https://github.com/wuzhizhe7273/os-switch"
license=('MIT')
makedepends=('cargo')

build() {
    cd "$startdir"
    cargo build --release --locked
}

package() {
    cd "$startdir"
    install -Dm755 target/release/os-switch "$pkgdir/usr/bin/os-switch"
}
