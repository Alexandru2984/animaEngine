# Audit local de securitate și calitate — 15 octombrie 2026

Revizie analizată: `bbe5d13`…`a5dc05f` și curățenia de cod care îl urmează, pe ramura 1.5 (nepublicată; `Cargo.toml` spune încă 1.4.0). Platforma verificărilor: Linux, Rust 1.98, plus verificări încrucișate pentru Windows (`x86_64-pc-windows-gnu`) și FreeBSD și MSRV 1.88.

Auditul nu a găsit o cale prin care conținut din afară — un fișier tras pe ecran, o scenă sau un backup primit, un pachet Shimeji, clipboardul, o fereastră străină, un apel D-Bus — să execute cod, să scrie în afara locurilor aplicației sau să citească fișiere pe care utilizatorul nu le-a ales. A găsit două căi prin care aplicația își putea umple jurnalul fără limită și câteva lucruri de strâns, toate reparate mai jos. Absența unei demonstrații nu este o certificare a absenței problemelor.

**Domeniu și metodă**

Auditul a mers pe granițele de încredere, nu pe fișiere: pentru fiecare loc prin care intră date din afara procesului, codul care le citește a fost citit cap-coadă, cu limitele, căile și aritmetica lui. Au fost acoperite 68.378 de linii în 172 de fișiere Rust, plus lockfile-urile, `deny.toml`, manifestul Flatpak și lista lui de surse. Același pas a căutat cod mort și duplicat.

Nu include o campanie lungă de fuzzing (care urmează în aceeași rundă, pe căile noi), sanitizere pe codul nativ, auditul bibliotecilor sistemului (Mesa, NVIDIA, openh264) sau o rulare pe Windows real.

Priorități: **P1** = de reparat înainte de a considera sigur conținutul din surse străine; **P2** = problemă reală, dependentă de condiții; **P3** = întărire/întreținere.

## Verificări executate

| Verificare | Rezultat |
|---|---|
| `cargo audit` (lockfile principal, 1.329 advisories, baza din 14.10.2026) | Doar cele acceptate în `deny.toml`: `quick-xml` 0.30/0.39 (tranzitive, la build; dependența directă e 0.41, reparată), `paste`, `smartstring`, `ttf-parser` (nemenținute), `memmap2` 0.8 |
| `cargo audit --file fuzz/Cargo.lock` | Înainte: în plus `crossbeam-epoch` 0.9.18, `event-listener` 5.4.1, `memmap2` 0.9.10, `anyhow` 1.0.102, `webbrowser` 1.2.1. După: același set ca principalul |
| `cargo deny check` | advisories, bans, licenses, sources: ok |
| `cargo machete` | Nicio dependență directă nefolosită |
| Inventar `unsafe` | 51 de apariții în 13 fișiere; `clippy::undocumented_unsafe_blocks` pe Linux și Windows: 5 fără justificare, acum 0 |
| Suprafața de panic (`unwrap_used`, `expect_used`, `indexing_slicing`, `panic`, `unreachable`) | 179 de locuri în codul care nu e de test; cele de pe căile cu date din afară citite unul câte unul |
| Elemente publice fără consumator / folosite doar de teste | 4 / 20; vezi „Cod mort” |
| Blocuri duplicate (≥ 10 linii semnificative) | 12 înainte, 3 după; vezi „Cod duplicat” |
| `cargo test`, ambele configurații | 908 și 894 de teste trecute |
| clippy `-D warnings` (Linux, ambele configurații; Windows; FreeBSD), rustdoc `-D warnings`, MSRV 1.88 | Trecut |

## Constatări

**1. P2 — Un script putea umple jurnalul fără limită. Reparat.**

`print` și `debug` din Rhai rămăseseră pe handlerul implicit, `println!` pe stdout, fără plafon. Limita de 10.000 de operații pe tick nu oprește un `loop { print(...) }`: la 60 de tick-uri pe secundă și șiruri de până la 4 KiB, un script putea scrie sute de MB pe secundă — în jurnalul sistemului, pentru o aplicație pornită la login. Scripturile vin din biblioteca utilizatorului (o scenă partajată nu le duce), dar un script copiat de pe internet e scenariul firesc.

Acum merg în logul de nivel debug (vizibil doar la cerere), tăiate la 200 de caractere, cel mult 8 linii pe rulare (`scripting::log_printed`). Test: `a_print_loop_goes_to_the_log_a_few_lines_a_run`.

**2. P2 — O suprafață care eșua scria un avertisment la fiecare cadru. Reparat.**

`Render error: …` se scria la fiecare cadru pe calea X11, pe suprafața principală Wayland și pe fiecare monitor suplimentar: o eroare persistentă (`SurfaceError::Other` pe unele drivere) însemna 60 de linii pe secundă. Pe Wayland, în plus, `Outdated` nu era reconfigurat deloc — se repeta la nesfârșit — iar un `Timeout`, tranzitoriu, era avertisment, nu cadru sărit.

Acum `renderer::ErrorOnce` spune fiecare eroare o dată și numără repetările (spuse când un cadru reușește sau vine altă eroare), câte unul pe suprafață; `Outdated` e reconfigurat ca `Lost`, `Timeout` merge la debug, ca pe X11.

**3. P3 — Clipboardul X11: o alocare de 64 MB ca semnal și un transfer fără termen. Reparat.**

Când răspunsul proprietarului clipboardului depășea plafonul, `convert` întorcea un buffer de 64 MB de zerouri, doar ca apelantul să-i măsoare lungimea. Un transfer în bucăți (`INCR`) avea termen doar pe bucată: un proprietar care trimitea bucăți mici, fiecare la timp, ținea firul ocupat oricât. Acum `convert` întoarce `Answer::{Bytes, TooBig, Nothing}`, fără nicio alocare pentru „prea mare”, iar `INCR` se încheie în 10 s. Verificat în rig: o imagine lipită de `xclip` intră ca personaj.

**4. P3 — Corpul notificărilor era citit ca markup. Reparat.**

După specificația Desktop Notifications, un server care declară `body-markup` interpretează corpul. Textele noastre sunt simple, dar pot veni din afară: un memento dintr-un backup restaurat, numele unui personaj dintr-o scenă partajată (înlocuit în `{name}`). Un `<a href=…>` sau `<img src=…>` trecea ca markup, iar un `&` banal îl strica. Corpul e acum escapat (`notify::plain_as_markup`); portalul primește oricum text. Verificat cu un server de notificări fals: `Tom &amp; Jerry &lt;b&gt;Cat Demo&lt;/b&gt;`.

**5. P3 — Un URI `file://` fără cale absolută devenea cale relativă. Reparat.**

`file://cat.png` (fără al treilea `/`) producea `cat.png`, rezolvat față de directorul de pornire al aplicației. Niciun file manager nu scrie așa; un proprietar de clipboard ostil putea. Parserul unic (drag-and-drop, clipboard, dialogul de fișiere) cere acum o cale absolută.

**6. P3 — Un lanț întreg de parsare URL, compilat degeaba. Scos.**

Funcția `links` a `egui-winit` era activă, deși aplicația nu deschide niciun link. O scoatere elimină 23 de crate-uri din build — `url`, `idna`, tot stack-ul ICU, `webbrowser` (care are istoric de advisories, ultimul pentru injecție prin `BROWSER`). `Cargo.lock` și `flatpak/cargo-sources.json` pierd exact acele intrări.

**7. P3 — Lockfile-ul fuzzerelor rămăsese în urmă. Reparat.** Versiunile cu advisories din tabelul de mai sus au trecut la cele reparate, pe care lockfile-ul principal le avea deja.

**8. P3 — Cinci blocuri `unsafe` fără justificare scrisă. Reparat.** Două în teste (`mkfifo`), un al doilea apel acoperit de comentariul primului în `schedule.rs` (Linux și Windows), `unsafe impl Send` pe Windows. Erau corecte; acum spun de ce.

## Verificat, fără probleme

- **D-Bus (`com.animaengine.Anima`).** Orice proces din sesiune îl poate apela; e limitat ca rată, primește doar numere și bool-uri, iar duratele sunt plafonate la intrare (snooze ≤ 24 h, focus ≤ 180 min) — `Snooze(4294967295)` nu depășește nimic.
- **Scene și backup-uri primite.** Numele intrărilor din arhivă trebuie să fie `assets/<cifre>/<nume simplu>.<extensie permisă>` — fără `..`, căi absolute sau fișiere ascunse; fiecare imagine e remapată la o intrare existentă; dezarhivarea merge într-un folder nou; scripturile, sunetele și monitoarele sunt scoase; replicile, plafonate; numele de fișier al unei scene păstrează doar litere, cifre și `-`. Un backup străin aplică setările așa cum sunt, dar niciuna nu e o cale: sunetele se rezolvă numai în bibliotecă (`resolve_library_asset`), limba și scena activă sunt comparate cu valori cunoscute.
- **Pachete Shimeji.** Fiecare `Image` din XML trece prin canonicalizare și verificarea prefixului `img/`; citiri plafonate; garda pentru FIFO; `quick-xml` 0.41.
- **Clipboard și drag-and-drop pe Wayland.** 64 KiB pentru liste de fișiere, 1 MiB text, 64 MiB imagini, 256 de căi, 4 drop-uri simultane.
- **Ferestrele altor aplicații (X11).** Toate citirile de proprietăți au lungimi mici și fixe (1–1024 de unități), niciodată `u32::MAX`.
- **Decodoarele.** Dimensiunile sunt verificate (≤ 4096) înaintea decodării, apoi cadrele și octeții plafonați; tabelele MP4 validate înainte de `read_sample` (P1 din septembrie, ține); sunetul decodat tăiat la 8 MiB, sub bugetul cache-ului, cu verificare la compilare (P1 din septembrie, ține); scope-ul Rhai readus la marcaj după fiecare rulare (P1 din septembrie, ține). Spritesheet: grila zero și celulele goale refuzate, offseturile rămân în imagine prin construcție. Miniatura unei scene: limite de dimensiune și alocare în decodor. Recunoașterea după conținut (runda 35) poate alege doar între cele patru formate compilate.
- **Indexările din parsere.** Cele 179 de locuri de panic au fost citite pe căile cu date din afară: antetele cache-ului sunt tablouri de mărime fixă, parcurgerea NAL își verifică limitele (și are fuzzer), `%XX` verifică lungimea, indicii calendarului sunt fixați cu `clamp`; `expect`/`panic` sunt în funcțiile de fuzz, unde panic-ul e scopul.
- **Portalul.** Răspunsurile `Response` sunt primite printr-un `zbus::Proxy` legat de proprietarul numelui `org.freedesktop.portal.Desktop`; un alt proces nu le poate injecta.
- **Logurile.** Fișierele utilizatorului apar redactate (`redact_path`); căile complete rămase sunt ale aplicației (directoarele de date, fontul CJK).
- **Procese copil.** Aplicația nu pornește niciunul; variabila setată pentru laptopurile hibride (`VK_LOADER_DRIVERS_DISABLE`) nu ajunge deci la alte programe, iar fără funcția `links` nu mai există nici calea teoretică prin care ar fi pornit un browser.

## Cod mort

Patru elemente publice nu le folosea nimeni, iar alte 16 le foloseau doar testele. Elementele publice ale unei biblioteci scapă de avertismentul `dead_code`, așa că trebuiau căutate:

- **Scoase**, cu testele care le verificau doar pe ele: `App::clear_warning` (comentariul din panou trimitea la ea, deși avertismentele sunt închise de `config_watch`), trei iconițe, `AppConfig::windows_normalised` (documentația pretindea că o folosesc submeniul tray-ului, selectorul de ferestre și `Activate()` — niciunul), `Copied::single_name`, `MonitorInfo::fallback_single`, `scene_file::is_backup`, `ScriptHost::forget`, `LIBRARY_EXTENSIONS`.
- **`LIBRARY_EXTENSIONS` și testul ei nu păzeau nimic.** Scanarea bibliotecii folosește `LibraryKind::from_extension`; testul compara constanta cu o listă scrisă de mână. Acum un test cere ca biblioteca să ia exact ce ia un drop.
- **Mutate sub `#[cfg(test)]`**, ca ajutoare de test: `contrast_ratio` și `relative_luminance`, `can_undo`/`can_redo`, `Entity::contains_point`, `active_state`, `is_compiled`, `fully_dismissed`, `delete_entity`/`duplicate_entity`.

## Cod duplicat

Douăsprezece blocuri de cel puțin zece linii semnificative apăreau de două ori. Nouă au fost unite:

- **Desenarea egui peste sprite-uri**, o copie în fiecare renderer, e acum `surface_egui::paint_over`.
- **Cele cinci personaje demo** au un constructor (`demo_character`), iar cele cinci generatoare împart începutul și salvarea cadrelor (`demo::prepare`, `save_frame`).
- **Pregătirea unei cereri către portal** e `portal::desktop_portal`.
- **Câmpul procentual din Inspector** e `percent`.

Rămân trei, lăsate cu intenție: argumentele panoului în cele două bucle (plumbing între backend-uri), pregătirea comună a două metode care împrumută câmpuri diferite din `App`, și listele de parametri ale panourilor.

## Adăugat pe 19.10.2026

Pe 16.10 RustSec a marcat `zvariant` (RUSTSEC-2026-0370), `zbus_names`
(-0371) și `zvariant_derive` (-0372) ca nemenținute: au fost mutate în
`zbus` 6.0. Job-ul `cargo audit` a picat pe primul push de după. Sunt
accesibile la rulare (serializarea D-Bus, prin `zbus::zvariant`), dar
fără defect cunoscut. Versiunile 5.x vin prin `zbus` 5, folosit și de
`ksni` 0.3, iar 4.x prin `accesskit_unix` 0.13 (egui 0.31). Un `zbus` 6
doar pentru noi ar adăuga o a treia copie și nu ar scoate niciuna.
Acceptate în `ci.yml` și `deny.toml`, cu justificarea acolo; de scos
când `ksni` și stiva egui/accesskit trec pe `zbus` 6.

## Riscuri rămase

- **H.264 se decodează în proces, de openh264 (C).** E documentat în modelul de amenințare; decodarea într-un proces separat e în lista de amânări.
- **Copiile vechi ale `quick-xml`** sunt tranzitive, la build, și nu primesc date din afară.
- **Acțiunile GitHub nu sunt fixate prin SHA** (în lista de amânări).
- **Decodoarele nu rulează într-un sandbox** (seccomp, Landlock).
