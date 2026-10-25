# Pregătire de produs — ce lipsește ca animaEngine să poată fi vândut

25 octombrie 2026, pe codul de la `a34b520` (1.5, nelansat). Nu e un aviz
juridic: unde e vorba de brevete, licențe sau mărci, scrie ce am verificat
și ce trebuie întrebat un avocat.

## Pe scurt

**Ce e deja la nivel de produs:** aplicația e stabilă (soak-uri de 2 ore pe
ambele backenduri, memorie plată), are zece limbi, cititor de ecran,
navigare din tastatură, pachete Linux reproductibile cu atestare de
proveniență, un audit de securitate și fuzzing pe fiecare cititor de
fișiere, și nu face niciun apel de rețea. Consumă puțin: pe laptopul tău,
cinci personaje în mișcare iau 6,9 % din timpul GPU-ului la 30 de cadre pe
secundă, iar cu scena oprită ~0,1 %.

**Ce blochează vânzarea astăzi (P0):**

1. **Brevetele H.264** (și AAC). Decodorul video e compilat din sursă, deci
   licența de brevete pe care Cisco o plătește pentru binarul *lor* nu se
   aplică. Pentru un produs plătit trebuie o decizie — variantele sunt mai
   jos.
2. ~~**Licențele terțe nu sunt în pachete.**~~ Rezolvat după acest audit
   (`84cfbd2`): `data/THIRD-PARTY-LICENSES.txt`, scris din `Cargo.lock`,
   e în fiecare pachet și în aplicație (Appearance → Despre).
3. **Windows.** Piața aplicațiilor de tip „desktop pet” e în mare parte pe
   Windows. Are acum installer și ZIP la fiecare release, iar sub Wine se
   instalează, rulează și se dezinstalează. N-a rulat pe un Windows real și
   nu e semnat (SmartScreen avertizează la fiecare descărcare).
4. **Decizia de business** — ce se vinde, la ce preț, pe ce canal, sub ce
   licență. Codul e MIT: oricine îl poate compila și redistribui. E a ta.
5. **Release-urile nu sunt publice.** 1.2.1, 1.3.0 și 1.4.0 sunt drafturi
   pe GitHub; public, ultima versiune e 1.2.0.

**Important (P1):** traducerile sunt automate în nouă limbi din zece; nu
există actualizări automate în afara managerelor de pachete; lipsește un
trailer. Personajele incluse sunt tot desenate de cod, dar redesenate
(runda 44) și acum zece (runda 45) — un artist ar ridica nivelul, nu mai
e o lipsă. Site-ul, pagina de magazin și suportul există ca draft (runda
44), de publicat de tine.

**Stare la 26 octombrie 2026** (`4317cb9` și după): rezolvate de la audit —
licențele terțe și „Despre” (43), suportul: raport de diagnostic, FAQ,
formulare de issue (44), site, politică de confidențialitate și fișă de
magazin (44), personajele redesenate și încă cinci, iconiță nouă, grafică
de magazin, ecran de bun venit la prima pornire (45).

## 1. Legal

### 1.1 Brevete: H.264 și AAC

- **H.264.** `openh264` e compilat din sursă (`openh264-sys2`, feature-ul
  `source`) și legat static în fiecare pachet. Cisco plătește redevențele
  pentru binarul pe care îl distribuie *ea*, descărcat de utilizator de la
  Cisco — de aceea Firefox descarcă exact acel binar la prima folosire.
  Un decodor compilat de noi și vândut nu e acoperit. Notă deja existentă în
  `Cargo.toml` și `docs/threat-model.md` („Patent note”).
- **AAC.** `rodio` aduce `symphonia` cu codecul AAC și containerul MP4,
  intenționat: un script poate reda sunetul unui MP4 folosit ca personaj.
  AAC are și el un pool de brevete.
- **Celelalte formate** pe care le citește aplicația — PNG, GIF, WebP, JPEG,
  WAV, Ogg Vorbis, FLAC, MP3 — nu ridică probleme de brevete active
  cunoscute (brevetele MP3 au expirat în 2017).

Variante, de la cea mai simplă:

| Variantă | Ce înseamnă | Pentru | Contra |
|---|---|---|---|
| A. Fără MP4 în buildurile vândute | `--no-default-features --features audio`, iar AAC scos din rodio | zero expunere, zero cod nou | se pierde MP4 ca personaj; GIF/WebP rămân |
| B. Decodorul sistemului | MP4 citit printr-un `ffmpeg` instalat de utilizator, într-un proces separat | nu distribuim niciun decodor; izolare mai bună (era în backlog-ul de hardening) | depinde de ce are sistemul (Fedora fără RPM Fusion n-are H.264); pe Windows nu există `ffmpeg` implicit |
| C. Binarul Cisco la cerere | `openh264` cu feature-ul `libloading`; un buton „Activează MP4” descarcă binarul Cisco, verificat după hash | acoperit de licența Cisco, ca în browsere | o conexiune de rețea, opțională, într-o aplicație care promite zero rețea; doar H.264 (AAC rămâne de rezolvat separat) |
| D. Licență proprie | licențiere directă de la pool | păstrează totul | cost și formalități; decizie juridică |

Recomandarea mea, de verificat cu un avocat: **A pentru prima versiune
vândută**, cu **B** ca extensie. **C** doar dacă MP4 e esențial pentru
clienți.

### 1.2 Licențe terțe

- Arborele livrat pe Linux (`cargo metadata --filter-platform`):
  - majoritatea crate-urilor sunt MIT și/sau Apache-2.0 (153 `MIT OR
    Apache-2.0`, 67 MIT, 40 `Apache-2.0 OR MIT`);
  - restul: BSD-2/3, ISC, Zlib, Unicode-3.0, CC0, Unlicense;
  - **MPL-2.0** la 14 crate-uri: `symphonia` și codecurile ei,
    `smartstring`, `option-ext`;
  - `epaint_default_fonts`: fonturi sub **OFL-1.1** și **Ubuntu Font
    Licence**.
- **Ce cer:**
  - MIT, BSD, ISC și Zlib: textul licenței și notița de copyright, cu
    binarul.
  - Apache-2.0: textul licenței și eventualele fișiere NOTICE.
  - OFL: textul licenței alături de fonturi.
  - MPL-2.0: să spui unde se găsește sursa acelor fișiere (crates.io e
    suficient, dacă e menționat).
- **Ce lipsește:** niciun pachet (.deb, AppImage, Flatpak, AUR) și nici
  aplicația nu includ aceste texte. Se rezolvă imediat după acest audit: un fișier
  generat din `Cargo.lock`, instalat cu fiecare pachet și afișat într-o
  secțiune „Despre” a aplicației.

### 1.3 Licența aplicației

- animaEngine e MIT. Oricine poate compila și vinde aceeași aplicație.
- Modele care funcționează cu MIT:
  - vânzarea buildurilor gata făcute și semnate (pentru Windows mai ales);
  - conținut plătit (pachete de personaje);
  - „plătește cât vrei”;
  - suport.
- O schimbare de licență pentru versiunile viitoare e posibilă doar pentru
  codul la care deții drepturile; contribuțiile externe trebuie verificate
  în istoric.

### 1.4 Numele

- „animaEngine” și „Anima” sunt nume frecvente. Înainte de un magazin,
  trebuie căutat în bazele de mărci (EUIPO, USPTO) și pe magazinele țintă.
- Domeniul pentru app id (`com.animaengine.Anima`) e încă deschis în
  [FLATHUB.md](../flatpak/FLATHUB.md): Flathub cere ca domeniul să fie al
  tău.

### 1.5 Confidențialitate

- Aplicația nu face niciun apel de rețea și nu trimite nimic.
  [threat-model.md](threat-model.md) arată unde scrie, iar strace o
  confirmă.
- O politică de confidențialitate devine astfel scurtă și ușor de respectat.
  Microsoft Store și majoritatea magazinelor o cer ca URL.

### 1.6 Conținut

- Personajele incluse (zece, din 1.5) sunt desenate de cod
  (`src/demo/`), deci sunt ale proiectului; la fel iconița și grafica de
  magazin, compuse din ele (`examples/store_pictures.rs`,
  `scripts/store-art.py`).
- Nu e inclus niciun sunet.
- Pachetele Shimeji le importă utilizatorul; aplicația nu le distribuie.

## 2. Platforme și distribuție

| Platformă | Stare | Ce lipsește pentru vânzare |
|---|---|---|
| Linux .deb / AppImage | construite în CI, reproductibile, cu atestare | licențele terțe; release-urile publicate |
| Flatpak | manifest Flathub gata | domeniul app id-ului; Flathub nu vinde (doar link de donații) |
| AUR | construiește (reparat în 1.5: `!lto`) | — |
| Windows | installer per utilizator (NSIS, 10 limbi) și ZIP portabil, construite la fiecare release; iconiță, informații de versiune, pornire la login; instalat, rulat și dezinstalat sub Wine | rulare pe Windows real; semnătură de cod |
| macOS | nu e suportat | stratul de ferestre (click-through, always-on-top) ar trebui scris |

**Canale de vânzare de luat în calcul** (prețurile și condițiile se schimbă
— de verificat la zi):

- **itch.io**: Linux și Windows, „plătește cât vrei”, fără cerințe de
  semnare.
- **Steam**: aplicațiile de tip desktop pet se vând acolo; are o taxă per
  aplicație și suportă Linux.
- **Gumroad / Lemon Squeezy**: plăți și chei de licență, dacă vrei
  build-uri doar pentru cumpărători.
- **Microsoft Store**: MSIX; semnarea o face magazinul.

**Semnarea pe Windows:**

- certificat OV/EV de la o autoritate de certificare;
- Azure Trusted Signing;
- pentru proiecte open source, SignPath Foundation.

Fără semnătură, SmartScreen avertizează la fiecare descărcare.

## 3. Produsul

- **Personaje incluse.** Zece, desenate de cod cu contur, lumină,
  umbră și ochi care clipesc, câte douăsprezece cadre (rundele 44–45). Un
  artist ar putea face mai mult — personaje cu personalitate, animații de
  mers și de somn —, comandate sau licențiate, cu drepturi de
  redistribuire. Importul Shimeji rămâne un avantaj.
- **Traduceri.** Nouă limbi din zece sunt traduse automat, cu un glosar și
  teste de consecvență. Pentru piețele țintă trebuie o recitire de către un
  vorbitor nativ.
- **Prima impresie.** La prima intrare în modul de editare, un ecran de
  bun venit: ce personaje, limba, pornirea la login (runda 45); apoi turul
  în trei pași. What's new și indiciile sunt verificate pas cu pas. Pagina
  de magazin, capturile la 1920×1080 și grafica pentru Steam și itch.io
  există (`site/`, `docs/store-listing.md`); lipsește un videoclip scurt.
- **Suport.** Formulare de issue pe GitHub, `docs/faq.md` (cu problemele
  cunoscute, de exemplu aprobarea scurtăturilor pe GNOME), `SUPPORT.md`
  (runda 44). Un e-mail de suport e decizia ta.
- **Actualizări.** Fără rețea, nu există actualizări din aplicație. Pe
  Linux le fac managerele de pachete și Flathub; AppImage poate primi
  informații zsync (AppImageUpdate); pe Windows, manual sau prin magazin.
- **Rapoarte de erori.** Instantaneele de recuperare sunt locale și nu se
  trimite nimic. Din runda 44, Despre → „Copiază raportul de diagnostic”
  pune în clipboard versiunea, sistemul și ultimele avertismente, pe care
  utilizatorul le trimite singur.

## 4. Calitate rămasă de verificat

- Lista din [gnome-checklist-1.5.md](gnome-checklist-1.5.md), pe hardware
  real: sunet, Orca, tray-ul AppIndicator.
- KDE Plasma pe Wayland: ambele căi (runda 46).
- Un soak de 7 zile ([soak-testing.md](soak-testing.md)).
- Windows real: pornire, click-through, tray, pornire la login, mai multe
  monitoare, DPI.

## 5. Priorități

| # | Ce | De ce | Cine | Efort |
|---|---|---|---|---|
| P0 | Licențe terțe în pachete și „Despre” | obligație legală la orice distribuție | făcut (`84cfbd2`) | — |
| P0 | Decizia H.264/AAC | expunere la brevete în buildurile vândute | tu decizi; eu implementez | mic–mediu |
| P0 | Windows: rulare reală, semnare | piața principală; installerul e făcut | tu (homelab, certificat), eu (ce iese la rulare) | mic–mediu |
| P0 | Model de business și canal | totul depinde de el | tu | — |
| P0 | Publicarea release-urilor | utilizatorii văd 1.2.0 | tu | mic |
| P1 | Personaje desenate | primul lucru pe care îl vede un cumpărător | redesenate și zece (44–45); un artist, opțional | — |
| P1 | Recitirea traducerilor | calitate percepută | vorbitori nativi | mediu |
| P1 | Pagina de produs, capturi, video | vânzarea în sine | pagină și capturi făcute (44–45); trailer în runda 46; publicarea e a ta | mic |
| P1 | Canal de suport, FAQ | încredere | făcut (44); e-mailul e al tău | — |
| P1 | Căutare de marcă, domeniu | evită o redenumire mai târziu | tu | mic |
| P2 | Actualizări AppImage (zsync) | comoditate | eu | mic |
| P2 | Raport de eroare copiat de utilizator | suport | făcut (44) | — |
| P2 | macOS | piață nouă | eu | mare |

## 6. Decizii care îți aparțin

1. Ce vinzi: build-uri gata făcute, conținut, suport, sau o combinație.
2. Unde: itch.io, Steam, magazinul tău, Microsoft Store.
3. Licența versiunilor viitoare: rămâne MIT sau nu.
4. H.264/AAC: varianta A, B, C sau D — amânată pe 25 octombrie 2026,
   până la părerea unui avocat.
5. Numele și domeniul.
6. Ce personaje: comandate, licențiate, sau doar import.

