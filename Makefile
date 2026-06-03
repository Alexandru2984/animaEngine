# animaEngine — install / uninstall helpers.
#
# `make install` lays the release binary down at $(BINDIR), copies the
# .desktop / icon / AppStream files to the right XDG-spec locations, and
# refreshes the desktop database. `make uninstall` reverses that.
#
# Override PREFIX / DESTDIR for staging into a package build (used by
# the .deb and AppImage recipes in 8.2 / 8.3).
#
#   make install                            # /usr/local
#   make install PREFIX=$$HOME/.local        # per-user
#   make install DESTDIR=/tmp/anima-stage PREFIX=/usr  # for packagers

PREFIX  ?= /usr/local
DESTDIR ?=

BINDIR     := $(DESTDIR)$(PREFIX)/bin
APPDIR     := $(DESTDIR)$(PREFIX)/share/applications
ICONDIR    := $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps
ICONROOT   := $(DESTDIR)$(PREFIX)/share/icons/hicolor
METAINFODIR:= $(DESTDIR)$(PREFIX)/share/metainfo
ICON_SIZES := 16 24 32 48 64 128 256

BINARY := target/release/anima_engine
APPID  := com.animaengine.Anima

.PHONY: all release install uninstall validate appimage deb flatpak clean-build icons

all: release

release:
	cargo build --release --locked

# Build a self-contained AppImage in build/. Downloads linuxdeploy on
# first run; cached for subsequent builds.
appimage:
	scripts/build-appimage.sh

# Build a .deb package via cargo-deb. Reads [package.metadata.deb] in
# Cargo.toml (assets, depends, description, …) set up in Etapa 8.1.
# Requires `cargo install cargo-deb` once per machine.
deb:
	scripts/build-deb.sh

# Build a Flatpak bundle via flatpak-builder. See flatpak/README.md for
# the one-time SDK install. Output: build/com.animaengine.Anima.flatpak.
flatpak:
	scripts/build-flatpak.sh

clean-build:
	rm -rf build/AppDir build/animaEngine-*.AppImage build/anima-engine_*.deb \
	       build/com.animaengine.Anima.flatpak build/flatpak-*

install: $(BINARY) icons
	install -Dm0755 $(BINARY)                              $(BINDIR)/anima-engine
	install -Dm0644 data/anima-engine.desktop              $(APPDIR)/anima-engine.desktop
	install -Dm0644 data/anima-engine.svg                  $(ICONDIR)/anima-engine.svg
	@for size in $(ICON_SIZES); do \
		install -Dm0644 build/icons/$$size/anima-engine.png \
		  $(ICONROOT)/$${size}x$${size}/apps/anima-engine.png; \
	done
	install -Dm0644 data/$(APPID).metainfo.xml             $(METAINFODIR)/$(APPID).metainfo.xml
	@if [ -z "$(DESTDIR)" ] && command -v update-desktop-database >/dev/null 2>&1; then \
		update-desktop-database -q $(APPDIR) || true; \
	fi
	@if [ -z "$(DESTDIR)" ] && command -v gtk-update-icon-cache >/dev/null 2>&1; then \
		gtk-update-icon-cache -q -t -f $(ICONROOT) 2>/dev/null || true; \
	fi
	@echo "Installed under $(PREFIX)."
	@echo "Run: anima-engine"

# Rasterize the SVG into the discrete hicolor sizes. Idempotent; runs
# automatically as part of `make install`, and triggered explicitly
# by `scripts/build-deb.sh` before cargo-deb reads the assets.
icons:
	scripts/render-icons.sh

uninstall:
	rm -f $(BINDIR)/anima-engine
	rm -f $(APPDIR)/anima-engine.desktop
	rm -f $(ICONDIR)/anima-engine.svg
	@for size in $(ICON_SIZES); do \
		rm -f $(ICONROOT)/$${size}x$${size}/apps/anima-engine.png; \
	done
	rm -f $(METAINFODIR)/$(APPID).metainfo.xml
	@if command -v update-desktop-database >/dev/null 2>&1; then \
		update-desktop-database -q $(APPDIR) || true; \
	fi
	@if command -v gtk-update-icon-cache >/dev/null 2>&1; then \
		gtk-update-icon-cache -q -t -f $(ICONROOT) 2>/dev/null || true; \
	fi

# Lint the metadata files before shipping. Both validators are widely
# packaged; missing tools become warnings, not failures.
validate:
	@if command -v desktop-file-validate >/dev/null 2>&1; then \
		desktop-file-validate data/anima-engine.desktop && \
		  echo "✓ .desktop file valid"; \
	else \
		echo "skip: desktop-file-utils not installed"; \
	fi
	@if command -v appstreamcli >/dev/null 2>&1; then \
		appstreamcli validate --no-net data/$(APPID).metainfo.xml && \
		  echo "✓ AppStream metadata valid"; \
	else \
		echo "skip: appstream/appstreamcli not installed"; \
	fi
