# Audit local de securitate și calitate — 12 septembrie 2026

Revizie analizată: `7111bed` — `Wire quit, save, delete and duplicate on native Wayland`. Pachet: `anima_engine 1.1.0`. Platforma verificărilor: Linux, Rust 1.96.0. Arborele Git era curat înaintea auditului.

Auditul a identificat probleme reale de disponibilitate și integritate. Nu a demonstrat execuție arbitrară de cod, escaladare de privilegii sau exfiltrare. Absența unei demonstrații nu este o certificare a absenței acestor probleme.

**Domeniu și limite**

Au fost examinate manual căile sensibile: decodarea asseturilor și cache-ul, scripturile Rhai, audio, importul Shimeji, rezolvarea căilor, scrierile de fișiere, hot-reload, lifecycle, diferențele dintre backenduri, IPC, fragmentele unsafe și configurațiile CI/packaging. Căutările de duplicări și simboluri fără consumatori au acoperit sursele Rust. Dependențele au fost verificate în ambele lockfile-uri.

Acesta este un audit al sursei și al dependențelor, cu trei probe locale controlate. Nu include o campanie prelungită de fuzzing, sanitizere pentru tot codul nativ, testare GUI pe toate platformele, audit complet al bibliotecilor sistemului sau examinarea fiecărei linii din toate dependențele. Riscurile statice de mai jos sunt etichetate separat de reproduceri.

Priorități: **P1** = remediere prioritară înainte de a considera sigur conținutul din surse care nu sunt de încredere; **P2** = problemă relevantă, dependentă de condiții; **P3** = întreținere/documentație. Acestea sunt priorități pentru proiect, nu scoruri CVSS.

**Verificări executate**

| Verificare | Rezultat |
|---|---|
| `cargo test --locked --all-targets` | 538 teste trecute; smoke-urile benchmarkurilor au trecut |
| `cargo test --locked --no-default-features --all-targets` | 523 teste trecute; smoke-urile benchmarkurilor au trecut |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Trecut |
| `cargo fmt --all -- --check` | Trecut |
| `cargo test --locked --doc` | Trecut; 0 doctesturi |
| `cargo machete` | Nu a raportat dependențe directe neutilizate |
| `cargo audit --json` | 4 potriviri de vulnerabilitate: 2 advisories × 2 versiuni quick-xml; alte 4 avertismente |
| `cargo audit --file fuzz/Cargo.lock --json` | 6 potriviri de vulnerabilitate și 6 avertismente |
| `cargo deny check` | Trecut cu politica actuală, care include excluderi explicite |
| Probe suplimentare | 3 reproduceri confirmate, descrise mai jos |

Baza RustSec consultată avea 1.243 advisories, commit `b50980aad8b8f14f77e25a97b32dd94bf008b0af`, ultima actualizare raportată 9 septembrie 2026. Scanarea cargo-audit nu a folosit excluderile din CI.

**1. P1 — Un MP4 foarte mic poate provoca panic în aplicație. Reprodus.**

Locuri: [video_loader.rs:116](../src/animation/video_loader.rs#L116), [input.rs:324](../src/app/input.rs#L324). Cauza imediată: `mp4 0.14.0`, `src/track.rs:456`, calculul chunkului împarte la `samples_per_chunk` fără să excludă zero.

Un fișier de **1.686 bytes** a trecut `pre_validate_dropped_file`, apoi apelul complet `load_asset(AssetType::Video, …)` a produs `attempt to divide by zero`. Proba a interceptat panic-ul în harness pentru a păstra testul controlat. Calea de încărcare din aplicație nu are acea interceptare; la drop, decodarea este apelată din tratarea inputului. La pornire poate împiedica încărcarea, iar într-un worker poate întrerupe workerul.

Limitele de 200 MiB pe fișier, numărul maxim de samples și limita RGBA nu previn acest caz: eroarea apare în demuxer, înaintea decodării cadrului.

Remediere: validați tabelele MP4 înainte de `read_sample`, inclusiv numărul de samples per chunk și consistența tabelelor; corectați sau înlocuiți demuxerul. Rulați parsarea și decodarea conținutului din surse care nu sunt de încredere într-un proces cu limite de memorie/timp. `catch_unwind` poate oferi reziliență la panic-uri Rust, dar nu izolează OOM, abort-uri sau erori de memorie native. Adăugați regresie pentru containerul malformat, nu doar pentru conversia NALU.

Observație suplimentară statică: aceeași dependență alocă `vec![0; sample_size]` înaintea citirii payloadului (`track.rs:548`). Un `sample_size` declarat în container nu este echivalent cu mărimea reală a fișierului. Nu am executat o probă care să epuizeze memoria; validați dimensiunea și intervalul fiecărui sample înainte de alocare.

**2. P1 — Scope-ul Rhai crește de la un cadru la altul. Reprodus.**

Locuri: [scripting.rs:491](../src/scripting.rs#L491), [entity.rs:223](../src/entity.rs#L223).

`run_ast_with_scope` păstrează declarațiile de la nivelul superior în scope-ul reutilizat. Codul presupune în comentariu că un `let` de la nivelul superior este eliminat la sfârșitul execuției; proba a arătat contrariul: un script cu o singură variabilă locală și o actualizare de poziție a ajuns la **118 intrări după 100 de tick-uri** — 18 intrări gazdă și 100 de declarații acumulate.

Impact: creștere persistentă a memoriei și costului căutărilor. Este suficient și un script obișnuit cu variabile locale; nu necesită o buclă infinită. Limita de 10.000 operații pe tick nu plafonează acumularea dintre tick-uri. Eliminarea scope-urilor entităților șterse nu rezolvă cazul unei entități active.

Remediere: păstrați explicit numai starea permisă între cadre. De exemplu, înregistrați lungimea scope-ului după instalarea variabilelor gazdă și eliminați declarațiile adăugate după execuție, inclusiv pe eroare, păstrând actualizările intenționate ale poziției și `state`. Adăugați limite pentru variabile și memoria persistentă. Testați multe tick-uri consecutive cu declarații locale și cu scripturi editate în timpul rulării.

**3. P1 — Audio este decodat fără plafon înaintea verificării cache-ului. Reprodus.**

Locuri: [audio.rs:321](../src/audio.rs#L321), [audio.rs:242](../src/audio.rs#L242).

`decoder.collect()` construiește integral `Vec<f32>`. Limita de 4 MiB se aplică fișierului comprimat. Proba cu un FLAC de **23.790 bytes**, 100 secunde, stereo/48 kHz, a produs **38.400.000 bytes** de samples — peste bugetul de 32 MiB al cache-ului. Proba a numărat samples în loc să le rețină, folosind același decoder, fără să necesite un dispozitiv audio.

Mai mult, dacă un sunet depășește singur bugetul, codul golește cache-ul și apoi îl inserează oricum. Bugetul nu este o limită strictă. În aplicație, încărcarea se face sincron din tick-ul entității, deci costul de decodare afectează interfața atunci când există ieșire audio disponibilă.

Remediere: plafonați numărul de samples în timpul decodării, derivați o durată maximă din sample rate și canale și respingeți sunetul care singur depășește bugetul. Mutați decodarea într-un worker cu coadă și buget limitate.

**4. P1 — Audio limitează pornirile pe tick, dar nu vocile active. Confirmat prin inspecția codului; fără test de stres audio.**

Locuri: [audio.rs:57](../src/audio.rs#L57), [audio.rs:267](../src/audio.rs#L267).

Fiecare pornire clonează întregul buffer de samples și îl adaugă mixerului. `MAX_VOICES_PER_TICK = 8` se resetează la fiecare tick, iar cooldown-ul de 150 ms permite pornirea repetată a aceluiași sunet înainte ca instanța anterioară să termine. Cache-ul de 32 MiB nu contabilizează copiile deținute de mixer.

Impact: multe sunete lungi suprapuse pot consuma multă memorie și CPU chiar dacă fiecare sunet individual ar fi acceptabil. În plus, hărțile `failed` și `last_played` nu au o politică de evacuare; cereri cu nume diferite se pot acumula în sesiuni lungi.

Remediere: limită globală de voci simultane, contabilizarea memoriei bufferelor aflate în redare, identificarea și oprirea vocilor, durată maximă și curățarea istoricului. Partajarea bufferelor între voci reduce memoria, dar nu înlocuiește limita de voci/CPU.

**5. P2 — Hot-reload poate suprascrie modificări făcute în timp ce workerul decodează. Confirmat prin inspecția fluxului.**

Locuri: [hot_reload.rs:37](../src/app/hot_reload.rs#L37), [hot_reload.rs:78](../src/app/hot_reload.rs#L78), [hot_reload.rs:131](../src/app/hot_reload.rs#L131).

`config_dirty` este verificat înainte de pornirea workerului. Când rezultatul sosește, este aplicat înaintea acelei verificări, înlocuind configurația și scena. Dacă utilizatorul mută sau adaugă o entitate în intervalul de decodare, rezultatul vechi poate suprascrie acele editări; nu există verificarea unei revizii locale la aplicare.

Remediere: capturați generația configurației/scenei la pornirea workerului și aplicați rezultatul numai dacă generația locală nu s-a schimbat. În caz contrar amânați, respingeți sau gestionați explicit conflictul. Test de regresie: worker întârziat controlat, editare locală, apoi rezultat livrat.

**6. P2 — Numele temporar pentru scrieri atomice nu diferențiază threadurile. Risc static, nereprodus în audit.**

Locuri: [util.rs:72](../src/util.rs#L72), [util.rs:135](../src/util.rs#L135).

Fișierul temporar este identificat prin cale și PID. Două scrieri concurente ale aceluiași asset/cache din același proces pot deschide cu truncate același temporar. `rename` atomic nu face atomică întreaga secvență atunci când writerii împart același inode temporar. Hot-reload și încărcările inițiate din UI reprezintă o combinație de analizat.

Remediere: temporar unic pentru fiecare operație, creat exclusiv (`create_new`/echivalent), curățare pe toate erorile și, dacă este necesar, serializare per destinație. Pentru promisiunea de durabilitate la power-loss, sincronizați și directorul după rename; sincronizarea fișierului singur nu este întreaga garanție.

**7. P2 — Reimportul Shimeji șterge cadrele precedente înainte să știe că noua secvență este validă. Risc static, nereprodus în audit.**

Locuri: [shimeji/mod.rs:617](../src/shimeji/mod.rs#L617), [shimeji/mod.rs:657](../src/shimeji/mod.rs#L657).

La reimport se apelează `remove_stale_frames` înaintea validării și copierii tuturor referințelor. Dacă pose-urile nu mai rezolvă imagini valide sau apare o eroare de copiere, operația poate eșua după pierderea cadrelor importate anterior. Destinația bazată pe slug poate fi comună unor packuri distincte cu același nume normalizat.

Remediere: construiți și validați importul într-un director temporar separat; înlocuiți importul anterior numai după succes și păstrați-l până la finalizarea tranzacției. Folosiți o identitate de pack care tratează explicit coliziunile de nume.

**8. P2 — Protecțiile secvențelor PNG au neconcordanțe. Observație statică, fără demonstrație de OOM.**

Locuri: [loader.rs:127](../src/animation/loader.rs#L127), [png_sequence.rs:47](../src/animation/png_sequence.rs#L47), [png_sequence.rs:107](../src/animation/png_sequence.rs#L107), [png_sequence.rs:124](../src/animation/png_sequence.rs#L124).

Validatorul sondează primele 1.000 de PNG-uri în ordinea directorului. Loaderul colectează până la 1.001, sortează și păstrează apoi 600. Prin urmare, un fișier din setul decodat poate lipsi din setul validat; `load_single_png` nu repetă limita de dimensiuni. Separat, workerii rezervă bugetul comun după decodarea cadrului, deci limita de bytes reținuți nu limitează memoria tranzitorie a tuturor decodărilor concurente.

Remediere: un singur set de fișiere selectate pentru validare și decodare, verificare pe fiecare fișier efectiv decodat și rezervarea memoriei estimate înainte de decode. Limitați explicit concurența în funcție de buget. Nu raportați limita RGBA ca plafon al RSS-ului procesului.

**Dependențe cu probleme cunoscute**

În `Cargo.lock` principal, `quick-xml 0.30.0` și `0.39.2` sunt potrivite cu:

- [RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html): verificarea atributelor poate avea cost pătratic; remediat în 0.41.0.
- [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html): alocări neplafonate la rezolvarea namespace-urilor cu NsReader; remediat în 0.41.0.

Dependența directă pentru Shimeji este deja `quick-xml 0.41.0`. Copia 0.39.2 vine prin `wayland-scanner`, un proc-macro. Copia 0.30.0 vine prin `zbus_xml` / `zbus-lockstep` / infrastructura AT-SPI. Graful și utilizările examinate nu demonstrează că XML-ul unui pack importat ajunge la aceste versiuni vechi. Prezența în lockfile trebuie raportată, fără a echivala automat cu exploatarea aplicației de către un asset.

`memmap2 0.8.0`, adus de `xkbcommon`, are [RUSTSEC-2026-0186](https://rustsec.org/advisories/RUSTSEC-2026-0186.html), un avertisment de unsoundness în operații pe intervale de memorie; remedierea upstream este în 0.9.11. Nu a fost demonstrat un apel vulnerabil accesibil din asseturile aplicației. Mai apar avertismente de lipsă de întreținere pentru `paste 1.0.15`, `smartstring 1.0.1`, `ttf-parser 0.25.1`; acestea nu sunt, prin ele însele, dovezi de exploatare.

**Lockfile-ul de fuzzing este rămas în urmă.** Pe lângă quick-xml, `fuzz/Cargo.lock` conține `crossbeam-epoch 0.9.18` cu [RUSTSEC-2026-0204](https://rustsec.org/advisories/RUSTSEC-2026-0204.html), remediat în 0.9.20, și `webbrowser 1.2.1` cu [RUSTSEC-2026-0257](https://rustsec.org/advisories/RUSTSEC-2026-0257.html), remediat în 1.2.2. Mai sunt avertismente pentru `anyhow 1.0.102`, `event-listener 5.4.1`, `memmap2 0.8.0` și `0.9.10`. Aceste constatări privesc graful harnessului; nu le atribui automat binarului livrat. Actualizați și auditați separat acest lockfile.

`cargo deny check` trece deoarece [deny.toml:28](../deny.toml#L28) acceptă explicit mai multe advisories. Justificările trebuie revizuite după schimbarea dependențelor sau utilizărilor; un rezultat verde cu excluderi nu înseamnă zero advisories.

OpenH264 inclus de `openh264-sys2 0.9.6` declară versiunea nativă **2.6.0**. [CVE-2025-27091 / advisory Cisco](https://github.com/cisco/openh264/security/advisories/GHSA-m99q-5j7x-7m9x) afectează 2.5.0 și mai vechi și este remediat în 2.6.0: nu îl raportez ca vulnerabilitate prezentă aici. Rămâne suprafața de atac a unui decoder nativ care rulează în proces; cargo-audit nu demonstrează siguranța întregului cod nativ.

**Cod duplicat și cod fără consumatori de producție**

Există logică paralelă pentru meniuri, bibliotecă și preseturi în [app/outcomes.rs:31](../src/app/outcomes.rs#L31) și [wayland/run.rs:1402](../src/wayland/run.rs#L1402). Este duplicare arhitecturală între backenduri, nu dovada unui fișier copiat accidental. Merită extrasă logica asupra scenei în funcții comune; păstrați distincte numai operațiile specifice platformei.

Scanarea exactă a blocurilor a găsit și o secvență comună de aproximativ 15 linii relevante în `ui/egui_renderer.rs:127` și `wayland/egui_render.rs:165`. Aceasta este o verificare euristică, nu o analiză exhaustivă a clonelor semantice.

O divergență concretă: backendul nativ Wayland nu are mecanismul de polling/worker pentru hot-reload al configurației pe care îl are `app/hot_reload.rs`. Nu am găsit echivalentul `try_reload`, al verificării mtime sau al `check_hot_reload` în run-loop-ul său. Tabelul README care marchează hot-reload drept disponibil pe ambele backenduri nu corespunde implementării examinate. Nu confundați acest lucru cu reîncărcarea scripturilor Rhai.

Candidați concreți pentru curățenie:

| Simbol | Situație observată | Acțiune |
|---|---|---|
| `ui::states::spinner`, `states.rs:121` | Niciun apel găsit în surse/teste/benchuri/exemple | Eliminați sau conectați la fluxul de încărcare |
| `KeyBindings::validate`, `bindings.rs:136` | Helper fără consumator, returnează mereu Ok; placeholder declarat | Eliminați sau implementați când există validare reală |
| `thumbnail_is_fresh`, `asset_library/mod.rs:597` | Folosit numai în test; verificarea runtime este repetată inline la linia 525 | Folosiți helperul în producție sau eliminați-l împreună cu testul redundant |
| `EasingCurve::frame_interval`, `anim.rs:164` | Apeluri numai în teste; runtime are propriul calcul | Centralizați dacă trebuie să reprezinte aceeași regulă |
| `ScriptHost::is_compiled` / `forget`, `scripting.rs:426/432` | Apeluri numai în teste | Restrângeți vizibilitatea sau justificați API-ul |
| `AppConfig::windows_normalised`, `config.rs:370` | Folosit numai în teste; comentariul descrie infrastructură pentru o etapă viitoare | Marcați explicit drept infrastructură neconectată |

Helperii utilizați exclusiv în teste nu sunt automat greșeli. Nici funcțiile publice pentru bench/fuzz, precum serializarea cache-ului, nu sunt cod mort accidental. Vizibilitatea publică a modulelor explică de ce Clippy poate rămâne verde deși aplicația nu consumă unele API-uri.

**Documentație și packaging**

- `MAX_DECODED_ASSET_BYTES` este efectiv **256 MiB**, dar comentarii și threat-model îl descriu încă drept 512 MB. Corectați inclusiv calculele agregate asociate.
- Promisiunile absolute că un fișier malformat nu poate provoca OOM sunt prea puternice față de alocările native, tranzitorii și audio identificate.
- Manifestul Flathub indică încă [tag v0.9.0](../flatpak/com.animaengine.Anima.flathub.yml#L72), în timp ce crate-ul este 1.1.0. Fișierul se prezintă drept manifest de pregătire pentru submission; verificați-l înainte de publicare, nu presupuneți că reflectă versiunea curentă.
- Fuzzingul existent acoperă conversia AVCC/NALU, dar nu întregul demuxer MP4 care a produs panic-ul. Adăugați un target pentru container, plus testare repetată a scripturilor și decodare audio cu bugete.

**Ordine recomandată de lucru**

1. Corectați panic-ul MP4 și limitele dinaintea alocărilor demuxerului; limitați sau izolați procesarea video.
2. Eliminați acumularea scope-ului Rhai și fixați prin test persistența strictă a stării permise.
3. Impuneți limite audio în timpul decodării și pentru vocile active, nu doar pentru cache.
4. Protejați aplicarea rezultatelor hot-reload împotriva editărilor intervenite între timp.
5. Faceți scrierile concurente și reimporturile tranzacționale; armonizați selecția/validarea PNG.
6. Actualizați lockfile-ul de fuzzing, revizuiți excluderile RustSec și completați targeturile lipsă.
7. Reduceți logica duplicată între backenduri și eliminați helperii neutilizați justificați de inventar.

Nu au fost aplicate remedieri în codul aplicației și nu au fost schimbate dependențele. Testul temporar de audit a fost scos din `tests/`. Dovezile locale și sursa probelor sunt păstrate în `/tmp/anima-security-audit-2026-09-12/`; această locație poate fi curățată de sistem și nu este un artefact publicat. Singurul fișier adăugat în repository de audit este prezentul raport.
