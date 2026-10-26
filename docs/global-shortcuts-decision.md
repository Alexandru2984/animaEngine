# Scurtăturile globale — ce alegem

26 octombrie 2026. Un document de decizie: nimic nu e schimbat în cod
până nu alegi.

## Problema

Cele trei scurtături globale implicite sunt **Ctrl+Shift+A** (modul de
editare), **Ctrl+Shift+H** (ascunde) și **Ctrl+Shift+P** (pauză). O
scurtătură globală e luată din toate aplicațiile cât timp animaEngine
rulează. Toate trei sunt deja folosite de aplicații pe care oamenii le au
deschise toată ziua:

| Scurtătura | Unde e deja folosită |
|---|---|
| Ctrl+Shift+A | KDE: acțiunea standard **Deselect**, în toate aplicațiile KDE (Plasma o marchează ca ocupată când îi cerem s-o lege) · Firefox: **Add-ons** · VS Code: **Toggle block comment** · IDE-urile JetBrains: **Find Action** |
| Ctrl+Shift+H | Firefox: **Library / History** · VS Code: **Replace in files** |
| Ctrl+Shift+P | VS Code: **Command Palette** (cea mai folosită comandă din VS Code) · Firefox: **New Private Window** |

S-a și întâmplat: cât timp rig-ul de test folosea sesiunea ta reală,
Ctrl+Shift+P a fost luat din VS Code.

## Criteriile

1. Să nu calce pe scurtăturile implicite ale desktopurilor: GNOME (citite
   din schemele instalate pe Ubuntu 26.04), KDE Plasma 6, Windows 11.
2. Să nu calce pe aplicațiile mari: browserele, VS Code, IDE-urile,
   terminalele.
3. Să nu împiedice scrisul. Pe Windows, **AltGr este Ctrl+Alt**: o
   scurtătură Ctrl+Alt+literă fură o literă cu diacritice pe tastaturile
   care o au pe AltGr — în poloneză ą ć ę ł ń ó ś ź ż (A C E L N O S X Z),
   în germană @ € µ (Q E M), € pe E în franceză, spaniolă și italiană, iar
   pe US-International (folosită mult în Olanda) á é í ó ú ñ ö ș.a. — inclusiv
   **ö pe P**.
4. Să urmeze ce spun platformele: KDE și-a mutat în Plasma 6 propriile
   scurtături globale de pe Ctrl+Alt pe **Meta**, tocmai pentru că „pot
   suprascrie scurtăturile aplicațiilor, fiindcă nu folosesc tasta Meta”.
5. Literele să rămână ușor de ținut minte: A, H, P.

## Familiile verificate

| Familia | Desktopuri | Aplicații | Scris (AltGr) | Verdict |
|---|---|---|---|---|
| Ctrl+Shift+literă (acum) | GNOME: liber (în afară de emoji, Ctrl+Shift+E/U) · KDE: Deselect (A) | **multe** (tabelul de sus) | fără probleme | slab |
| Ctrl+Alt+literă | KDE: Ctrl+Alt+L blochează ecranul · GNOME: Ctrl+Alt+T terminal | puține | **fură litere pe Windows** (ą pe A, ö pe P) | nu |
| Ctrl+Alt+Shift+literă | liber peste tot | aproape liber | pe Windows = AltGr+Shift: **Ą** pe A, Ö pe P (US-Intl) | nu, pentru A și P |
| **Super+Alt+literă** (Win+Alt pe Windows) | GNOME: doar S (cititorul de ecran), 8, =, − · KDE: K, L (aranjamentul tastaturii), săgețile · Windows: B, D, K, săgețile, cifrele, Game Bar (G, R, T, M, PrtScn); H doar cât e deschisă dictarea vocală | **niciuna** dintre cele verificate | nu atinge AltGr | **da** |

## Recomandarea

| Acțiunea | Acum | Propunere |
|---|---|---|
| Modul de editare | Ctrl+Shift+A | **Super+Alt+A** |
| Ascunde / arată | Ctrl+Shift+H | **Super+Alt+H** |
| Pauză | Ctrl+Shift+P | **Super+Alt+P** |

Pe Windows, Super e tasta Windows. Singura atingere rămasă: pe Windows,
Win+Alt+H mută focusul pe tastatură **doar cât timp e deschisă dictarea
vocală**. Dacă vrei să eviți și asta, Super+Alt+O („overlay”) e liber
peste tot, dar se ține minte mai greu.

## Ce costă

- **Cei care fac upgrade păstrează ce au.** Config-ul salvează toate
  scurtăturile, inclusiv pe cele implicite, deci cine are deja un
  `config.toml` rămâne cu Ctrl+Shift+A/H/P. Noile valori ajung la
  instalările noi. Variantă: la prima pornire a 1.5, cine are exact cele
  trei vechi implicite le primește pe cele noi, cu un rând în What's new.
  Asta ar schimba însă și scurtăturile cuiva care le-a ales conștient
  identice cu cele implicite — rar, dar posibil.
- **GNOME și KDE țin minte legăturile.** Portalul păstrează ce a aprobat
  utilizatorul, iar ce propune aplicația e doar o sugestie la prima
  legare. Cine a aprobat deja Ctrl+Shift+A o păstrează până o schimbă din
  setările desktopului.
- **De actualizat:** valorile implicite, testele, pasul 3 al turului și
  What's new (10 limbi), README-ul, documentația, FAQ-ul, site-ul și
  fișa de magazin, plus trailerul, dacă vreuna dintre scene arată
  scurtătura (nu arată).
- **Laptopuri fără tasta Super:** foarte rare printre țintele noastre
  (Linux, Windows); scurtăturile rămân oricum schimbabile din
  Keybindings.

## Surse

- GNOME 50: valorile implicite din `/usr/share/glib-2.0/schemas/*.gschema.xml`
  (Ubuntu 26.04).
- KDE Plasma 6, scurtăturile standard (Deselect = Ctrl+Shift+A):
  [kconfig `kstandardshortcut.cpp`](https://invent.kde.org/frameworks/kconfig/-/raw/master/src/gui/kstandardshortcut.cpp);
  mutarea pe Meta: [KWin MR 2481](https://invent.kde.org/plasma/kwin/-/merge_requests/2481);
  [Common Keyboard Shortcuts](https://docs.kde.org/stable_kf6/en/khelpcenter/fundamentals/kbd.html),
  [Plasma desktop shortcuts](https://docs.kde.org/stable_kf6/en/plasma-desktop/plasma-desktop/shortcuts.html);
  verificat și în sesiunea Plasma 6 din runda 46 (dialogul de scurtături).
- Windows 11: [Keyboard shortcuts in Windows](https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-windows-dcc61a57-8ff0-cffe-9796-cb9706c75eec).
- Firefox: [Keyboard shortcuts](https://support.mozilla.org/en-US/kb/keyboard-shortcuts-perform-firefox-tasks-quickly).
- Chrome: [Chrome keyboard shortcuts](https://support.google.com/chrome/answer/157179).
- VS Code: [Keyboard shortcuts for Linux (PDF)](https://code.visualstudio.com/shortcuts/keyboard-shortcuts-linux.pdf).
