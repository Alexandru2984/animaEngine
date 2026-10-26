# Lansarea 1.5 — ce e verificat și ce rămâne de făcut

26 octombrie 2026, pe `main`. Pachetele au fost construite ca în CI și
instalate curat; mai jos, ce a ieșit și pașii care sunt ai tăi.

## Ce am verificat (runda 47)

Construit exact ca `release.yml`, din `git archive` al commit-ului, în
containere: Linux pe **Ubuntu 22.04** (cum rulează job-ul de release),
cu `cargo-deb 3.7.0` și `cargo-auditable 0.7.5`; Windows pe **Ubuntu
24.04**.

| Pachet | Rezultat |
|---|---|
| `.deb` (9,3 MB) | se instalează și se dezinstalează curat pe Ubuntu 22.04, 24.04 și Debian 12; dependențele se rezolvă singure; `anima-engine --version` merge |
| `.deb`, în rig | testul bun-venitului 17/17 pe Wayland nativ și pe X11, dintr-un director din afara depozitului; personajele generate cu `walk/`, `fall/`, `drag/` |
| AppImage (12 MB) | testul bun-venitului 17/17 pe X11 |
| Installer Windows (6,7 MB) și ZIP (8,8 MB) | sub Wine: instalare per utilizator, intrare de dezinstalare în registru, scurtătură în Start, aplicația pornește, tray-ul apare, personajele cu stări în `%APPDATA%`, dezinstalare completă |

**Găsit și reparat:**

- **Job-ul de release ar fi picat la build-ul Windows.** Pe Ubuntu 22.04,
  mingw-w64 (GCC 10) nu poate lega `.exe`-ul (`undefined reference to
  __mingwthr_key_dtor`). CI-ul obișnuit verifică Windows doar cu clippy,
  fără linkare, deci n-ar fi ieșit la iveală decât la primul tag.
  `release.yml` are acum un job Windows separat, pe 24.04, iar sumele de
  control se fac după ce sunt gata toate pachetele.
  `scripts/build-windows.sh` refuză din start un mingw prea vechi.
- **lintian:** o pagină de manual (`anima-engine.1`, și în Makefile și
  în AUR) și trei override-uri justificate în pachet (binarul păstrează
  simbolurile pentru backtrace-urile din rapoartele de crash; „zlib” e
  de fapt miniz_oxide; `SingleMainWindow` e o cheie standard).
- Jurnalul de pornire (și deci raportul de diagnostic) spunea „X11
  XGrabKey” pe Windows; acum spune `RegisterHotKey`.

**lintian, rezolvat în runda 48:** câmpul Maintainer are adresa ta
(`984.alexmihai@proton.me`), iar build-ul scrie un `changelog.Debian`
pentru versiunea construită. Cu lintian-ul de pe Ubuntu 22.04 nu mai
rămâne nimic; cel de pe sistemul tău raportează doar un `groff-message`
pe care nu-l pot reproduce — `man --warnings` pe aceeași pagină trece
curat.

## Ce ai de făcut, în ordine

1. ~~Scurtăturile globale~~ — făcut în runda 48: Super+Alt+A/H/P la un
   config nou; un config vechi își păstrează scurtăturile.
2. ~~Adresa pentru pachete~~ — făcut în runda 48.
3. **Commit-ul de versiune** (al tău, ca de obicei):
   - `Cargo.toml` → `1.5.0` (și `Cargo.lock`, prin `cargo update -p anima_engine`);
   - `data/com.animaengine.Anima.metainfo.xml` → un `<release version="1.5.0" date="…">`;
   - `CHANGELOG.md`: `[Unreleased]` → `[1.5.0] — data`;
   - `README.md`: rândul de stare și comenzile de instalare (acum `1.4.0`);
   - `docs/release-notes/v1.5.0.md` există deja — o recitire.
4. **Tag-ul** `v1.5.0` pe `main` și push → `release.yml` construiește
   (Linux pe 22.04, Windows pe 24.04), atestă și deschide un **draft**.
   Verifici draft-ul și îl publici.
5. **Draft-urile vechi** (1.2.1, 1.3.0, 1.4.0) încă nu sunt publice;
   public, ultima e 1.2.0. Le publici sau le ștergi.
6. **După lansare:**
   - AUR: `pkgver=1.5.0` și `sha256sums` din arhiva tag-ului
     (`packaging/aur/PKGBUILD`);
   - Flathub: `tag: v1.5.0` și `commit:` în
     `flatpak/com.animaengine.Anima.flathub.yml` (acum `v1.4.0`);
   - GitHub Pages: workflow-ul „pages”, pornit manual;
   - magazinele: imaginile din `site/media/store/`, trailerul din
     `build/trailer/` (MP4 pentru Steam, WebM pentru itch.io);
     descrierea din [store-listing.md](store-listing.md).
7. **Windows real**, înainte de a-l vinde: kit-ul din
   `packaging/windows/test-kit/` pe un VM din homelab; semnarea codului
   (fără ea, SmartScreen avertizează la fiecare descărcare).
