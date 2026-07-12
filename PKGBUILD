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

prepare() {
    cd "${pkgname}-${pkgver}"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "${pkgname}-${pkgver}"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --release --frozen
}

package() {
    cd "${pkgname}-${pkgver}"
    install -Dm755 target/release/os-switch "$pkgdir/usr/bin/os-switch"
}
