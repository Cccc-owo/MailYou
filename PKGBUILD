pkgname=mailyou-git
pkgver=0.0.0.r0.gunknown
pkgrel=1
pkgdesc="Desktop-first mail client built with Vue, Electron, and Rust"
arch=('x86_64')
url="https://github.com/Cccc-owo/MailYou"
license=('MIT')
options=('!lto')
depends=('gtk3' 'nss' 'libsecret' 'libxss' 'libxtst' 'xdg-utils' 'at-spi2-core')
makedepends=('git' 'nodejs' 'npm' 'rust')
source=("git+$url.git")
sha256sums=('SKIP')

pkgver() {
  cd "$srcdir/MailYou"
  git describe --long --tags --always --abbrev=7 2>/dev/null | sed 's/^v//;s/-/.r/;s/-/./'
}

prepare() {
  cd "$srcdir/MailYou"
  npm ci
}

build() {
  cd "$srcdir/MailYou"
  npm run pack:linux:dir
}

package() {
  cd "$srcdir/MailYou"

  install -dm755 "$pkgdir/opt/mailyou"
  cp -a release/linux-unpacked/. "$pkgdir/opt/mailyou/"

  install -dm755 "$pkgdir/usr/bin"
  cat > "$pkgdir/usr/bin/mailyou" <<'EOF'
#!/bin/sh
exec /opt/mailyou/mailyou "$@"
EOF
  chmod 755 "$pkgdir/usr/bin/mailyou"

  install -dm755 "$pkgdir/usr/share/applications"
  install -dm755 "$pkgdir/usr/share/pixmaps"
  install -dm755 "$pkgdir/usr/share/icons/hicolor/256x256/apps"
  install -dm755 "$pkgdir/usr/share/icons/hicolor/scalable/apps"
  install -Dm644 src/assets/logo.png "$pkgdir/usr/share/pixmaps/mailyou.png"
  install -Dm644 src/assets/logo.png "$pkgdir/usr/share/icons/hicolor/256x256/apps/mailyou.png"
  install -Dm644 src/assets/logo.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/mailyou.svg"

  cat > "$pkgdir/usr/share/applications/mailyou.desktop" <<'EOF'
[Desktop Entry]
Name=MailYou
Comment=Desktop-first mail client
Exec=/usr/bin/mailyou
Terminal=false
Type=Application
Icon=mailyou
Categories=Network;Email;Office;
MimeType=x-scheme-handler/mailto;x-scheme-handler/mailyou;
StartupWMClass=mailyou
EOF

  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/mailyou/LICENSE"
}
