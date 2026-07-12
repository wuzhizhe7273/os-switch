# Maintainer: Your Name <your@email.com>

pkgname=os-switch
pkgver=0.2.1
pkgrel=1
pkgdesc="双系统快速切换工具，支持休眠切换和直接切换"
arch=('x86_64')
url="https://github.com/wuzhizhe7273/os-switch"
license=('MIT')
makedepends=('cargo')
source=("${pkgname}-${pkgver}.tar.gz::${url}/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

_build_dir() {
    if [[ -f "$startdir/Cargo.toml" ]]; then
        echo "$startdir"
    else
        echo "$srcdir/${pkgname}-${pkgver}"
    fi
}

build() {
    cd "$(_build_dir)"
    cargo build --release --locked
}

package() {
    cd "$(_build_dir)"
    install -Dm755 target/release/os-switch "$pkgdir/usr/bin/os-switch"
}
